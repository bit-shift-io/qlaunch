import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/app_cell_visual.dart';
import 'package:qlaunch/src/ui/raster_cache_identity.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';

/// One app in the grid: an icon and a label laid out per `Settings.layout`.
///
/// It takes plain values, not `LauncherState`, so it can be tested without a
/// backend. The icon is whatever the caller resolved; `null` draws a
/// placeholder.
class AppCell extends StatefulWidget {
  const AppCell({
    super.key,
    required this.item,
    required this.icon,
    required this.settings,
    this.folder = false,
    this.geometry,
    this.onTap,
    this.onLongPress,
    this.onPointerDown,
  });

  final Item item;
  final ImageProvider? icon;
  final Settings settings;

  /// Whether this cell is a folder tile. With no icon it draws a folder glyph
  /// instead of the app/shortcut placeholder.
  final bool folder;

  /// Measured once by the enclosing grid; standalone cells fall back to
  /// measuring their own label.
  final AppCellGeometry? geometry;

  /// Called only for an item that can be launched: one with an app that is
  /// still installed. For any other item a tap does nothing.
  final VoidCallback? onTap;

  /// Called for every item, launchable or not.
  final VoidCallback? onLongPress;

  /// Reports the pointer that lands on this cell. A listener is used rather
  /// than a recogniser so it cannot compete with the cell's own tap.
  final ValueChanged<int>? onPointerDown;

  /// The box `icon_size` dp square that holds the icon or its placeholder.
  static const iconKey = AppCellVisual.iconKey;
  static const labelKey = AppCellVisual.labelKey;
  static const placeholderKey = AppCellVisual.placeholderKey;

  // A legacy document can carry an uninstalled item until a successful app
  // listing removes it; while it is still here, draw it faded to say it
  // cannot be launched.
  static const _uninstalledOpacity = 0.4;

  @override
  State<AppCell> createState() => _AppCellState();
}

class _AppCellState extends State<AppCell> {
  /// A cached tile must not be captured while an `Image` is still decoding:
  /// the capture would freeze the empty frame, and the provider itself does
  /// not change when the frame arrives.
  bool _iconReady = false;
  bool _readyScheduled = false;
  ImageProvider? _readyFor;

  Item get item => widget.item;
  Settings get settings => widget.settings;

  bool get _launchable =>
      item.folderGroupId != null ||
      item.shortcut != null ||
      (item.app != null && !item.uninstalled);

  bool get _iconCacheable {
    final provider = item.uninstalled ? null : widget.icon;
    return provider == null || _iconReady;
  }

  @override
  void didUpdateWidget(covariant AppCell oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.icon, widget.icon)) {
      _iconReady = false;
      _readyFor = null;
    }
  }

  void _markIconReady(ImageProvider provider) {
    if (_iconReady && identical(_readyFor, provider)) return;
    if (_readyScheduled) return;
    _readyScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _readyScheduled = false;
      if (!mounted || !identical(widget.icon, provider)) return;
      setState(() {
        _iconReady = true;
        _readyFor = provider;
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    Widget cell = GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: _launchable ? widget.onTap : null,
      // Always claim the long press, even when there is no handler. With only a
      // tap recogniser registered, a tap recognises a press held past its
      // deadline and fires `onTap` on release, so resting a finger on a cell
      // would launch it. Claiming the press makes holding inert, which is also
      // what Plan 3 wants when the press opens App Settings.
      onLongPress: widget.onLongPress ?? () {},
      child: LayoutBuilder(builder: _content),
    );
    if (widget.onPointerDown != null) {
      cell = Listener(
        onPointerDown: (event) => widget.onPointerDown!(event.pointer),
        child: cell,
      );
    }
    if (item.uninstalled) {
      cell = Opacity(opacity: AppCell._uninstalledOpacity, child: cell);
    }
    // One cell repainting (an icon fading in, say) must not repaint its
    // neighbours, so give the grid's scroll a cheap per-cell layer boundary.
    return RepaintBoundary(child: cell);
  }

  Widget _content(BuildContext context, BoxConstraints constraints) {
    final layout = settings.layout;
    final geometry =
        widget.geometry ?? appCellGeometry(context, settings, constraints);
    final side = geometry.iconSide;
    final generation = RasterCacheScope.generationOf(context);

    Widget? iconBox = settings.iconsVisible
        ? AppCellVisual.iconWidget(
            item: item,
            icon: widget.icon,
            settings: settings,
            side: side,
            folder: widget.folder,
            onIconFrame: _markIconReady,
          )
        : null;
    Widget? label = settings.labelsVisible
        ? AppCellVisual.labelWidget(item: item, settings: settings)
        : null;
    final rasterMode = TileRasterCache.modeFor(
      iconsVisible: settings.iconsVisible,
      labelsVisible: settings.labelsVisible,
    );
    if (rasterMode == TileRasterMode.separate) {
      if (iconBox != null) {
        iconBox = RasterTile(
          cacheKey: appCellRasterKey(
            generation: generation,
            item: item,
            icon: widget.icon,
            settings: settings,
            geometry: geometry,
            kind: 'icon',
          ),
          captureReady: _iconCacheable,
          child: iconBox,
        );
      }
      if (label != null) {
        label = RasterTile(
          cacheKey: appCellRasterKey(
            generation: generation,
            item: item,
            icon: widget.icon,
            settings: settings,
            geometry: geometry,
            kind: 'label',
          ),
          label: item.label,
          child: label,
        );
      }
    }

    if (rasterMode != TileRasterMode.combined) {
      return AppCellVisual.frame(
        layout: layout,
        body: AppCellVisual.bodyFrom(
          layout: layout,
          iconBox: iconBox,
          label: label,
        ),
      );
    }

    // The warm path renders the same AppCellVisual with the same geometry.
    return RasterTile(
      cacheKey: appCellRasterKey(
        generation: generation,
        item: item,
        icon: widget.icon,
        settings: settings,
        geometry: geometry,
        kind: 'tile',
      ),
      captureReady: _iconCacheable,
      label: item.label,
      child: AppCellVisual(
        item: item,
        icon: widget.icon,
        settings: settings,
        folder: widget.folder,
        geometry: geometry,
        onIconFrame: _markIconReady,
      ),
    );
  }
}
