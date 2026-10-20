import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cell.dart';

/// A list of items drawn as the launcher's grid of [AppCell]s.
///
/// It is given the items rather than a group, so anything with a list of items
/// (a home page, a search result) gets the same cells and the same icon
/// handling. The grid geometry comes from [settings]: the column count, and the
/// icon size, which is scaled to physical pixels for the platform.
///
/// Icons come from [LauncherState], which caches the provider for the whole
/// session, so disposing and rebuilding this widget (as the pager does) does not
/// refetch anything.
class AppCells extends StatelessWidget {
  const AppCells({
    super.key,
    required this.state,
    required this.settings,
    required this.items,
    this.onItemTap,
    this.onItemLongPress,
  });

  final LauncherState state;

  /// The settings the grid is laid out and drawn with.
  final Settings settings;

  /// The items to draw, already in display order.
  final List<Item> items;

  /// Replaces the default tap, which launches the item's app. Called only for
  /// items that can launch: an item with no app, or whose app is uninstalled,
  /// is never passed here.
  final ValueChanged<Item>? onItemTap;

  /// Called for any item. With none given, a long-press does nothing.
  final ValueChanged<Item>? onItemLongPress;

  void _launch(Item item) {
    final app = item.app;
    if (app == null) return;
    unawaited(state.launchApp(app.package, app.activity));
  }

  @override
  Widget build(BuildContext context) {
    final portrait = MediaQuery.orientationOf(context) == Orientation.portrait;
    final columns = math.max(
      1,
      (portrait
              ? settings.gridColumnsPortrait
              : settings.gridColumnsLandscape)
          .toInt(),
    );
    // The bridge takes physical pixels; the setting is in dp.
    final sizePx =
        (settings.iconSize * MediaQuery.devicePixelRatioOf(context)).round();

    return GridView.builder(
      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: columns,
        // Square cells in every layout. A builder choice: the spec gives
        // no ratio, and a cell this shape holds an icon with its label
        // below or beside it.
        childAspectRatio: 1,
      ),
      itemCount: items.length,
      itemBuilder: (context, i) {
        final item = items[i];
        final app = item.app;
        // Only an installed app has an icon to fetch. A shortcut has no
        // app, and an uninstalled app no longer has an icon; both draw the
        // placeholder. A hidden or zero-sized icon is not worth a request.
        // TODO(plan-3): `Item.iconOverride` names a PNG under the config
        // root, but there is no bridge function to read its bytes yet, so
        // the platform icon is still shown for such an item.
        final wantsIcon =
            app != null &&
            !item.uninstalled &&
            settings.iconsVisible &&
            sizePx > 0;
        Widget cell(ImageProvider? icon) => AppCell(
          item: item,
          icon: icon,
          settings: settings,
          onTap: () => (onItemTap ?? _launch)(item),
          onLongPress: onItemLongPress == null
              ? null
              : () => onItemLongPress!(item),
        );
        if (!wantsIcon) {
          return KeyedSubtree(key: ValueKey(item.id), child: cell(null));
        }
        return FutureBuilder<ImageProvider?>(
          key: ValueKey(item.id),
          future: state.iconFor(app.package, app.activity, sizePx),
          builder: (context, snapshot) => cell(snapshot.data),
        );
      },
    );
  }
}
