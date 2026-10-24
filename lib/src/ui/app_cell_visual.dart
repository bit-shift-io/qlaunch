import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/label_style.dart';
import 'package:qlaunch/src/ui/raster_cache_identity.dart';

/// The visual body of one app cell: icon and label laid out per `Settings`.
///
/// It has no gestures, opacity, semantics or nested [RasterTile], so both the
/// live cell and an offscreen warmer render byte-identical pixels from the same
/// inputs and [AppCellGeometry].
class AppCellVisual extends StatelessWidget {
  const AppCellVisual({
    super.key,
    required this.item,
    required this.icon,
    required this.settings,
    required this.geometry,
    this.folder = false,
    this.onIconFrame,
  });

  final Item item;
  final ImageProvider? icon;
  final Settings settings;
  final bool folder;
  final AppCellGeometry geometry;

  /// Reports an icon frame or failure so the live cell can gate new captures.
  final ValueChanged<ImageProvider>? onIconFrame;

  static const iconKey = ValueKey('appCell.icon');
  static const labelKey = ValueKey('appCell.label');
  static const placeholderKey = ValueKey('appCell.placeholder');

  @override
  Widget build(BuildContext context) =>
      frame(layout: settings.layout, body: body(context));

  /// The icon-or-placeholder box, exactly `geometry.iconSide` dp square.
  static Widget iconWidget({
    required Item item,
    required ImageProvider? icon,
    required Settings settings,
    required double side,
    bool folder = false,
    ValueChanged<ImageProvider>? onIconFrame,
  }) {
    final provider = item.uninstalled ? null : icon;
    final placeholder = AppCellPlaceholder(
      side: side,
      item: item,
      folder: folder,
    );
    final opacity = (settings.iconAlpha / 100).clamp(0.0, 1.0);
    Widget image = provider == null
        ? placeholder
        : Image(
            image: provider,
            fit: BoxFit.contain,
            gaplessPlayback: true,
            frameBuilder: (context, child, frame, wasSynchronouslyLoaded) {
              if (frame != null) onIconFrame?.call(provider);
              return child;
            },
            errorBuilder: (context, error, stack) {
              onIconFrame?.call(provider);
              return placeholder;
            },
          );
    if (opacity < 1.0) image = Opacity(opacity: opacity, child: image);
    return SizedBox.square(key: iconKey, dimension: side, child: image);
  }

  static Widget labelWidget({required Item item, required Settings settings}) {
    return Text(
      item.label,
      key: labelKey,
      maxLines: 2,
      softWrap: true,
      overflow: TextOverflow.ellipsis,
      textAlign: settings.layout == LayoutStyle.list
          ? TextAlign.start
          : TextAlign.center,
      style: labelTextStyle(settings),
    );
  }

  /// Clip/pad/align wrapper shared by the live tile and the separator tiles.
  static Widget frame({required LayoutStyle layout, required Widget body}) {
    return ClipRect(
      child: Padding(
        padding: layout == LayoutStyle.list
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

  Widget body(BuildContext context) => bodyFrom(
    layout: settings.layout,
    iconBox: settings.iconsVisible
        ? iconWidget(
            item: item,
            icon: icon,
            settings: settings,
            side: geometry.iconSide,
            folder: folder,
            onIconFrame: onIconFrame,
          )
        : null,
    label: settings.labelsVisible
        ? labelWidget(item: item, settings: settings)
        : null,
  );

  static Widget bodyFrom({
    required LayoutStyle layout,
    required Widget? iconBox,
    required Widget? label,
  }) {
    if (layout == LayoutStyle.list) {
      return Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          ?iconBox,
          if (iconBox != null && label != null) const SizedBox(width: 10),
          if (label != null) Flexible(child: label),
        ],
      );
    }
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        ?iconBox,
        if (iconBox != null && label != null) const SizedBox(height: 2),
        if (label != null) Flexible(child: label),
      ],
    );
  }
}

/// The rounded placeholder drawn for a shortcut or missing icon.
class AppCellPlaceholder extends StatelessWidget {
  const AppCellPlaceholder({
    super.key,
    required this.side,
    required this.item,
    this.folder = false,
  });

  final double side;
  final Item item;
  final bool folder;

  @override
  Widget build(BuildContext context) {
    final icon = folder
        ? Icons.folder
        : switch (item.shortcut) {
      ShortcutPayload_LockScreen() => Icons.lock_outline,
      ShortcutPayload_Notifications() => Icons.notifications_none,
      ShortcutPayload_Command() => Icons.terminal,
      ShortcutPayload_Android() => Icons.link,
      null => Icons.apps,
    };
    return DecoratedBox(
      key: AppCellVisual.placeholderKey,
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
