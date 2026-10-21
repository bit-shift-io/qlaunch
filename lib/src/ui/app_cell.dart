import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/label_style.dart';
import 'package:qlaunch/src/ui/grid_geometry.dart';

/// One app in the grid: an icon and a label laid out per `Settings.layout`.
///
/// It takes plain values, not `LauncherState`, so it can be tested without a
/// backend. The icon is whatever the caller resolved; `null` draws a
/// placeholder.
class AppCell extends StatelessWidget {
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

  // An uninstalled item stays in the model so that reinstalling restores it
  // to its group; it is drawn faded to say it cannot be launched.
  static const _uninstalledOpacity = 0.4;

  bool get _launchable =>
      item.shortcut != null || (item.app != null && !item.uninstalled);

  @override
  Widget build(BuildContext context) {
    Widget cell = GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: _launchable ? onTap : null,
      // Always claim the long press, even when there is no handler. With only a
      // tap recogniser registered, a tap recognises a press held past its
      // deadline and fires `onTap` on release, so resting a finger on a cell
      // would launch it. Claiming the press makes holding inert, which is also
      // what Plan 3 wants when the press opens App Settings.
      onLongPress: onLongPress ?? () {},
      child: LayoutBuilder(builder: _content),
    );
    if (onPointerDown != null) {
      cell = Listener(
        onPointerDown: (event) => onPointerDown!(event.pointer),
        child: cell,
      );
    }
    if (item.uninstalled) {
      cell = Opacity(opacity: _uninstalledOpacity, child: cell);
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

    final iconBox = settings.iconsVisible ? _icon(side) : null;
    final label = settings.labelsVisible ? _label(list) : null;

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

    return ClipRect(
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
  }

  Widget _icon(double side) {
    final provider = item.uninstalled ? null : icon;
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
            errorBuilder: (context, error, stack) => placeholder,
          );
    // At full opacity the layer would be a no-op saveLayer, so skip it.
    if (opacity < 1.0) image = Opacity(opacity: opacity, child: image);
    return SizedBox.square(key: iconKey, dimension: side, child: image);
  }

  Widget _label(bool list) {
    return Text(
      item.label,
      key: labelKey,
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
