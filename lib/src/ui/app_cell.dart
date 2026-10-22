import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/label_style.dart';
import 'package:qlaunch/src/ui/grid_geometry.dart';
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
    this.onTap,
    this.onLongPress,
    this.onPointerDown,
  });

  final Item item;
  final ImageProvider? icon;
  final Settings settings;

  /// Called only for an item that can be launched: one with an app that is
  /// still installed. For any other item a tap does nothing.
  final VoidCallback? onTap;

  /// Called for every item, launchable or not.
  final VoidCallback? onLongPress;

  /// Reports the pointer that lands on this cell. A listener is used rather
  /// than a recogniser so it cannot compete with the cell's own tap.
  final ValueChanged<int>? onPointerDown;

  /// The box `icon_size` dp square that holds the icon or its placeholder.
  static const iconKey = ValueKey('appCell.icon');
  static const labelKey = ValueKey('appCell.label');
  static const placeholderKey = ValueKey('appCell.placeholder');

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
      item.shortcut != null || (item.app != null && !item.uninstalled);

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
    final width = constraints.hasBoundedWidth ? constraints.maxWidth : 100.0;
    final list = layout == LayoutStyle.list;

    // Cap the icon so it can never be wider than its cell; in the list layout
    // it must leave room for the label as well.
    var side = settings.iconSize.toDouble();
    side = math.min(side, list ? math.max(0, (width - 10) / 2) : width);
    if (constraints.hasBoundedHeight) {
      final textHeight = gridLabelHeight(context, settings);
      side = math.min(
        side,
        math.max(0, constraints.maxHeight - (list ? 10 : textHeight + 10)),
      );
    }

    final devicePixelRatio = MediaQuery.devicePixelRatioOf(context);
    final textScale = MediaQuery.textScalerOf(context)
        .scale(settings.labelSize.toDouble());
    final labelSignature = (
      item.label,
      width,
      side,
      list,
      settings.labelSize,
      settings.labelTypeface,
      settings.labelColour,
      settings.labelShadow,
      settings.labelShadowBlur,
      textScale,
      devicePixelRatio,
      constraints.hasBoundedHeight ? constraints.maxHeight : -1.0,
    );
    final iconSignature = (
      item.id,
      item.shortcut,
      item.uninstalled,
      widget.icon,
      side,
      settings.iconAlpha,
      devicePixelRatio,
    );

    Widget? iconBox = settings.iconsVisible ? _icon(side) : null;
    Widget? label = settings.labelsVisible ? _label(list) : null;
    if (TileRasterCache.mode == TileRasterMode.separate) {
      if (iconBox != null) {
        iconBox = RasterTile(
          signature: iconSignature,
          cacheable: _iconCacheable,
          child: iconBox,
        );
      }
      if (label != null) {
        label = RasterTile(
          signature: labelSignature,
          label: item.label,
          child: label,
        );
      }
    }

    final Widget body;
    if (list) {
      body = Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          ?iconBox,
          if (iconBox != null && label != null) const SizedBox(width: 10),
          if (label != null) Flexible(child: label),
        ],
      );
    } else {
      // A Column lays its children out in sequence, so the label cannot be
      // drawn over the icon (the old centred layout's bug).
      body = Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          ?iconBox,
          if (iconBox != null && label != null) const SizedBox(height: 2),
          if (label != null) Flexible(child: label),
        ],
      );
    }

    final content = ClipRect(
      child: Padding(
        padding: list
            ? const EdgeInsets.all(5)
            : const EdgeInsets.only(top: 6, bottom: 2),
        child: Align(
          alignment: layout == LayoutStyle.tiles
              ? Alignment.topCenter
              : Alignment.center,
          child: body,
        ),
      ),
    );
    if (TileRasterCache.mode != TileRasterMode.combined) return content;

    // Includes every input that changes the painted tile, so a font, size,
    // colour, shadow, text-scale or DPI change re-renders from the live widget.
    return RasterTile(
      signature: (
        layout,
        iconSignature,
        labelSignature,
        settings.iconsVisible,
        settings.labelsVisible,
      ),
      cacheable: _iconCacheable,
      label: item.label,
      child: content,
    );
  }

  Widget _icon(double side) {
    final provider = item.uninstalled ? null : widget.icon;
    final placeholder = _Placeholder(side: side, item: item);
    // `icon_alpha` is 0-100. Rust clamps it, and Opacity asserts its range, so
    // clamp here as well rather than trust the value to be sane.
    final opacity = (settings.iconAlpha / 100).clamp(0.0, 1.0);
    Widget image = provider == null
        ? placeholder
        : Image(
            image: provider,
            fit: BoxFit.contain,
            gaplessPlayback: true,
            frameBuilder: (context, child, frame, wasSynchronouslyLoaded) {
              if (frame != null) _markIconReady(provider);
              return child;
            },
            errorBuilder: (context, error, stack) {
              _markIconReady(provider);
              return placeholder;
            },
          );
    // At full opacity the layer would be a no-op saveLayer, so skip it.
    if (opacity < 1.0) image = Opacity(opacity: opacity, child: image);
    return SizedBox.square(key: AppCell.iconKey, dimension: side, child: image);
  }

  Widget _label(bool list) {
    return Text(
      item.label,
      key: AppCell.labelKey,
      maxLines: 2,
      softWrap: true,
      overflow: TextOverflow.ellipsis,
      textAlign: list ? TextAlign.start : TextAlign.center,
      // The same style the tab strip reads, so labels cannot drift apart.
      style: labelTextStyle(settings),
    );
  }
}

class _Placeholder extends StatelessWidget {
  const _Placeholder({required this.side, required this.item});

  final double side;
  final Item item;

  @override
  Widget build(BuildContext context) {
    final icon = switch (item.shortcut) {
      ShortcutPayload_LockScreen() => Icons.lock_outline,
      ShortcutPayload_Command() => Icons.terminal,
      ShortcutPayload_Android() => Icons.link,
      null => Icons.apps,
    };
    return DecoratedBox(
      key: AppCell.placeholderKey,
      decoration: BoxDecoration(
        color: Colors.white24,
        borderRadius: BorderRadius.circular(side * 0.2),
      ),
      child: Center(
        child: Icon(icon, size: side * 0.6, color: Colors.white70),
      ),
    );
  }
}
