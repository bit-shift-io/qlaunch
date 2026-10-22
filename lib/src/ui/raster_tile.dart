import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:qlaunch/src/ui/raster_cache_stats.dart';
import 'package:qlaunch/src/ui/raster_capture_scheduler.dart';
import 'package:qlaunch/src/ui/raster_image_store.dart';

export 'raster_image_store.dart'
    show RasterCacheKey, RasterImageLease, RasterImageStore;

enum TileRasterMode { none, separate, combined, automatic }

/// Production cache policy; selection of the measured experiments only.
class RasterCacheConfig {
  const RasterCacheConfig({
    this.maximumBytes = 64 << 20,
    this.maximumEntries = 4096,
    this.recentGenerations = 2,
    this.mode = TileRasterMode.automatic,
    this.visibleScheduler = false,
  });

  final int maximumBytes;
  final int maximumEntries;
  final int recentGenerations;
  final TileRasterMode mode;

  /// Rejected in Task 6; retained as explicit configuration only.
  final bool visibleScheduler;
}

/// Reads `--dart-define=TILE_RASTER=...`. Unknown values are a hard error in
/// debug/profile and fall back to automatic in release.
TileRasterMode tileRasterModeFromEnvironment({bool defaultAutomatic = false}) {
  const flag = String.fromEnvironment('TILE_RASTER');
  if (flag.isEmpty) {
    return defaultAutomatic ? TileRasterMode.automatic : TileRasterMode.none;
  }
  return switch (flag) {
    'none' => TileRasterMode.none,
    'separate' => TileRasterMode.separate,
    'combined' => TileRasterMode.combined,
    'automatic' => TileRasterMode.automatic,
    _ =>
      kReleaseMode
          ? TileRasterMode.automatic
          : throw ArgumentError.value(
              flag,
              'TILE_RASTER',
              'Unknown raster mode',
            ),
  };
}

/// Session cache shared across pages; Flutter owns the rendered image handles.
class TileRasterCache {
  static TileRasterMode mode = tileRasterModeFromEnvironment();

  /// The mode a cell should use for this appearance. `automatic` keeps the
  /// combined tile only when both icon and label are visible, and avoids sparse
  /// raster work when the label is hidden (icons already use Flutter's decoded
  /// cache). Tabs and headings stay cached whenever caching is enabled.
  static TileRasterMode modeFor({
    required bool iconsVisible,
    required bool labelsVisible,
  }) {
    if (mode != TileRasterMode.automatic) return mode;
    if (!labelsVisible) return TileRasterMode.none;
    return iconsVisible ? TileRasterMode.combined : TileRasterMode.separate;
  }

  static RasterImageStore store = RasterImageStore();

  /// Post-paint capture queue. Visible work uses it only when
  /// [immediateVisibleCapture] is false; speculative work always does.
  static RasterCaptureScheduler scheduler = RasterCaptureScheduler();
  static bool immediateVisibleCapture = true;
  static bool get enabled => mode != TileRasterMode.none;

  /// Startup configuration; safe before widgets mount.
  static void configure(RasterCacheConfig config) {
    scheduler.dispose();
    scheduler = RasterCaptureScheduler();
    immediateVisibleCapture = !config.visibleScheduler;
    // Release discoverable pixels from the previous store; displayed leases
    // keep their images until released, so nothing under a live RawImage is
    // disposed here.
    store.clear();
    store = RasterImageStore(
      maximumBytes: config.maximumBytes,
      maximumEntries: config.maximumEntries,
      recentGenerations: config.recentGenerations,
    );
    mode = config.mode;
  }

  static int get captures => store.statistics.captures;
  static RasterCacheStats get statistics => store.statistics;
  static void resetStatistics() => store.resetStatistics();

  /// Drops unpinned pixels now and marks displayed ones for removal on release.
  static void trimForMemoryPressure() => store.trimForMemoryPressure();
  static void reset() {
    mode = TileRasterMode.none;
    immediateVisibleCapture = true;
    scheduler.dispose();
    scheduler = RasterCaptureScheduler();
    store.clear();
    store.resetStatistics();
  }
}

/// Supplies one store/generation to tabs, cells and headings. Does not own it.
class RasterCacheScope extends InheritedWidget {
  const RasterCacheScope({
    super.key,
    required this.store,
    required this.generation,
    required super.child,
  });
  final RasterImageStore store;
  final Object generation;
  static RasterCacheScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<RasterCacheScope>();
  static Object generationOf(BuildContext context) =>
      maybeOf(context)?.generation ??
      TileRasterCache.store.currentGeneration ??
      0;
  @override
  bool updateShouldNotify(RasterCacheScope oldWidget) =>
      oldWidget.store != store || oldWidget.generation != generation;
}

/// Paints [child] once, then draws the full-resolution captured image instead.
/// Gestures and animated opacity remain outside the captured visual.
class RasterTile extends StatefulWidget {
  const RasterTile({
    super.key,
    required this.cacheKey,
    required this.child,
    this.label,
    this.captureReady = true,
    this.store,
  });
  final RasterCacheKey cacheKey;
  final Widget child;
  final String? label;

  /// Only gates new captures. A complete matching hit is already safe to show.
  final bool captureReady;
  final RasterImageStore? store;
  @override
  State<RasterTile> createState() => _RasterTileState();
}

class _RasterTileState extends State<RasterTile> {
  final _boundaryKey = GlobalKey(debugLabel: 'rasterTile');
  RasterImageLease? _shown;
  RasterImageStore? _shownStore;
  bool _captureScheduled = false;

  @override
  void dispose() {
    TileRasterCache.scheduler.cancelOwner(this);
    _show(null, null);
    super.dispose();
  }

  void _show(RasterImageLease? lease, RasterImageStore? store) {
    if (identical(lease, _shown)) return;
    final previous = _shown;
    _shown = lease;
    _shownStore = store;
    previous?.release();
  }

  @override
  Widget build(BuildContext context) {
    final scope = RasterCacheScope.maybeOf(context);
    final store = widget.store ?? scope?.store ?? TileRasterCache.store;
    final key = widget.cacheKey;
    if (!TileRasterCache.enabled) {
      _show(null, null);
      return widget.child;
    }
    if (_shown?.key != key ||
        !identical(_shownStore, store) ||
        _shown!.retired) {
      _show(store.acquire(key), store);
    }
    final shown = _shown;
    if (shown != null) {
      return LayoutBuilder(
        builder: (context, constraints) {
          final size = constraints.constrain(shown.logicalSize);
          if (!size.width.isFinite || !size.height.isFinite || size.isEmpty) {
            _show(null, null);
            return widget.child;
          }
          return SizedBox.fromSize(
            size: size,
            child: Semantics(
              label: widget.label,
              child: RawImage(
                image: shown.image,
                scale: shown.pixelRatio,
                fit: BoxFit.fill,
                filterQuality: FilterQuality.low,
              ),
            ),
          );
        },
      );
    }
    if (!widget.captureReady) return widget.child;
    _scheduleCapture(store, key);
    return RepaintBoundary(key: _boundaryKey, child: widget.child);
  }

  bool _isCurrent(RasterImageStore store, RasterCacheKey key) {
    if (!mounted || !TileRasterCache.enabled || !widget.captureReady) {
      return false;
    }
    final scope = RasterCacheScope.maybeOf(context);
    return key == widget.cacheKey &&
        identical(store, widget.store ?? scope?.store ?? TileRasterCache.store);
  }

  /// Captures the painted boundary and publishes it. Runs after paint, either
  /// directly (immediate) or from the scheduler.
  void _capture(RasterImageStore store, RasterCacheKey key) {
    _captureScheduled = false;
    if (!_isCurrent(store, key)) return;
    if (store.contains(key)) {
      setState(() {});
      return;
    }
    final boundary =
        _boundaryKey.currentContext?.findRenderObject()
            as RenderRepaintBoundary?;
    if (boundary == null || boundary.size.isEmpty) return;
    final size = boundary.size;
    final ratio = MediaQuery.devicePixelRatioOf(context);
    final watch = Stopwatch()..start();
    var failed = false;
    try {
      final image = boundary.toImageSync(pixelRatio: ratio);
      _show(store.insertAndAcquire(key, image, ratio, size), store);
    } catch (_) {
      failed = true;
    } finally {
      store.recordCapture(
        microseconds: watch.elapsedMicroseconds,
        failed: failed,
      );
    }
    if (mounted && !failed) setState(() {});
  }

  void _scheduleCapture(RasterImageStore store, RasterCacheKey key) {
    if (_captureScheduled) return;
    _captureScheduled = true;
    if (TileRasterCache.immediateVisibleCapture) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _capture(store, key));
      return;
    }
    TileRasterCache.scheduler.enqueueVisible(
      owner: this,
      key: key,
      isCurrent: () => _isCurrent(store, key),
      capture: () async => _capture(store, key),
    );
  }
}
