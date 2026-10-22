import 'dart:collection';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

/// How tiles are cached during the spike.
enum TileRasterMode {
  none,

  /// Cache the icon and the label independently, keeping the live layout.
  separate,

  /// Cache the finished tile (icon, label and their spacing) as one image.
  combined,
}

/// Throwaway spike: cache a painted tile, or its parts, as a GPU image.
///
/// The key includes everything that changes the pixels, so a font, icon-size,
/// colour, shadow, text-scale or DPI change misses the cache and captures
/// again. [mode] defaults to [TileRasterMode.none], keeping the normal widget
/// path for production and for the widget tests. Delete this file if the spike
/// loses.
class TileRasterCache {
  static TileRasterMode mode = switch (const String.fromEnvironment(
    'TILE_RASTER',
  )) {
    'separate' => TileRasterMode.separate,
    'combined' => TileRasterMode.combined,
    _ => TileRasterMode.none,
  };
  static int captures = 0;

  /// At most two or three style generations. A shadow-blur slider can ask for
  /// eleven; without a bound those images would add up to hundreds of MB.
  static const int maxEntries = 128;

  static final LinkedHashMap<Object, TileRasterEntry> _entries =
      LinkedHashMap<Object, TileRasterEntry>();

  static bool get enabled => mode != TileRasterMode.none;

  static TileRasterEntry? lookup(Object signature) {
    if (!enabled) return null;
    final entry = _entries.remove(signature);
    if (entry == null) return null;
    _entries[signature] = entry;
    return entry;
  }

  static void store(
    Object signature,
    ui.Image image,
    double pixelRatio,
    Size size,
  ) {
    final previous = _entries.remove(signature);
    if (previous != null) _retire(previous);
    _entries[signature] = TileRasterEntry(image, pixelRatio, size);
    captures++;
    while (_entries.length > maxEntries) {
      final oldest = _entries.keys.first;
      final entry = _entries.remove(oldest)!;
      _retire(entry);
    }
  }

  static void retain(TileRasterEntry entry) => entry.references++;

  static void release(TileRasterEntry entry) {
    entry.references--;
    if (entry.references <= 0 && entry.retired) entry.image.dispose();
  }

  static void _retire(TileRasterEntry entry) {
    entry.retired = true;
    if (entry.references <= 0) entry.image.dispose();
  }

  /// Disable and free every cached image. Callers must let the tree rebuild
  /// without raster tiles first, or a displayed image would be disposed under
  /// a live [RawImage].
  static void reset() {
    mode = TileRasterMode.none;
    captures = 0;
    for (final entry in _entries.values) {
      _retire(entry);
    }
    _entries.clear();
  }
}

class TileRasterEntry {
  TileRasterEntry(this.image, this.pixelRatio, this.size);

  final ui.Image image;
  final double pixelRatio;
  final Size size;
  int references = 0;
  bool retired = false;
}

/// Paints [child] once, then draws the captured image instead.
///
/// The gesture layer stays outside this widget, so taps and long presses keep
/// working when the live text is gone.
class RasterTile extends StatefulWidget {
  const RasterTile({
    super.key,
    required this.signature,
    required this.child,
    this.label,
    this.cacheable = true,
  });

  final Object signature;
  final Widget child;
  final String? label;

  /// False while an icon is still decoding: capturing now would freeze the
  /// empty frame.
  final bool cacheable;

  @override
  State<RasterTile> createState() => _RasterTileState();
}

class _RasterTileState extends State<RasterTile> {
  final GlobalKey _boundaryKey = GlobalKey(debugLabel: 'rasterTile');
  TileRasterEntry? _shown;
  bool _captureScheduled = false;

  @override
  void dispose() {
    _show(null);
    super.dispose();
  }

  void _show(TileRasterEntry? entry) {
    if (identical(entry, _shown)) return;
    final previous = _shown;
    _shown = entry;
    if (entry != null) TileRasterCache.retain(entry);
    if (previous != null) TileRasterCache.release(previous);
  }

  @override
  Widget build(BuildContext context) {
    if (!TileRasterCache.enabled || !widget.cacheable) {
      _show(null);
      return widget.child;
    }
    final entry = TileRasterCache.lookup(widget.signature);
    _show(entry);
    if (entry != null) {
      return LayoutBuilder(
        builder: (context, constraints) {
          final size = constraints.constrain(entry.size);
          if (!size.width.isFinite ||
              !size.height.isFinite ||
              size.width <= 0 ||
              size.height <= 0) {
            _show(null);
            return widget.child;
          }
          return SizedBox.fromSize(
            size: size,
            child: Semantics(
              label: widget.label,
              child: RawImage(
                image: entry.image,
                scale: entry.pixelRatio,
                fit: BoxFit.fill,
                filterQuality: FilterQuality.low,
              ),
            ),
          );
        },
      );
    }
    _scheduleCapture();
    return RepaintBoundary(key: _boundaryKey, child: widget.child);
  }

  void _scheduleCapture() {
    if (_captureScheduled) return;
    _captureScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _captureScheduled = false;
      if (!mounted || !TileRasterCache.enabled || !widget.cacheable) return;
      final signature = widget.signature;
      if (TileRasterCache.lookup(signature) != null) return;
      final boundary =
          _boundaryKey.currentContext?.findRenderObject()
              as RenderRepaintBoundary?;
      if (boundary == null || boundary.size.isEmpty) return;
      final size = boundary.size;
      final pixelRatio = MediaQuery.devicePixelRatioOf(context);
      try {
        final image = boundary.toImageSync(pixelRatio: pixelRatio);
        TileRasterCache.store(signature, image, pixelRatio, size);
      } catch (_) {
        return;
      }
      if (mounted && signature == widget.signature) setState(() {});
    });
  }
}
