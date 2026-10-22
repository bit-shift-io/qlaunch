import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cell.dart';
import 'package:qlaunch/src/ui/grid_geometry.dart';
import 'package:qlaunch/src/ui/scroll_edge_fade.dart';

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
    this.onItemPointerDown,
    this.embedded = false,
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

  /// Reports the pointer that lands on a cell, before an ancestor listener
  /// sees the same down event.
  final ValueChanged<int>? onItemPointerDown;

  /// A non-scrolling row inside the continuous group scroller.
  final bool embedded;

  void _launch(Item item) {
    final app = item.app;
    if (app != null && !item.uninstalled) {
      unawaited(state.launchApp(app.package, app.activity));
      return;
    }
    if (item.shortcut != null) {
      unawaited(state.launchShortcut(item.id));
    }
  }

  @override
  Widget build(BuildContext context) {
    final portrait = MediaQuery.orientationOf(context) == Orientation.portrait;
    final columns = math.max(
      1,
      (portrait ? settings.gridColumnsPortrait : settings.gridColumnsLandscape)
          .toInt(),
    );
    // The bridge takes physical pixels; the setting is in dp.
    final sizePx = (settings.iconSize * MediaQuery.devicePixelRatioOf(context))
        .round();

    return LayoutBuilder(
      builder: (context, constraints) {
        final rowExtent = embedded
            ? constraints.maxHeight
            : gridRowExtent(context, settings, constraints.maxWidth / columns);
        final tileFade =
            !embedded && settings.scrollFade == ScrollFadeStyle.tile;
        Widget buildTile(BuildContext context, int i) {
          final item = items[i];
          final app = item.app;
          // Only an installed app has a platform icon, and only a shortcut with
          // a stored icon has bytes to read. Everything else draws the
          // placeholder. A hidden or zero-sized icon is not worth a request.
          Future<ImageProvider?>? icon;
          ImageProvider? cached;
          if (settings.iconsVisible && sizePx > 0) {
            if (app != null && !item.uninstalled) {
              cached = state.cachedIcon(app.package, app.activity, sizePx);
              if (cached == null) {
                icon = state.iconFor(app.package, app.activity, sizePx);
              }
            } else if (item.iconOverride != null) {
              cached = state.cachedItemIcon(
                item.id,
                item.iconOverride!,
                sizePx,
              );
              if (cached == null) {
                icon = state.itemIcon(
                  item.id,
                  reference: item.iconOverride!,
                  sizePx: sizePx,
                );
              }
            }
          }
          Widget cell(ImageProvider? icon) => AppCell(
            item: item,
            icon: icon,
            settings: settings,
            onTap: () => (onItemTap ?? _launch)(item),
            onLongPress: onItemLongPress == null
                ? null
                : () => onItemLongPress!(item),
            onPointerDown: onItemPointerDown,
          );
          final Widget tile;
          if (icon == null) {
            tile = KeyedSubtree(key: ValueKey(item.id), child: cell(cached));
          } else {
            tile = FutureBuilder<ImageProvider?>(
              key: ValueKey(item.id),
              future: icon,
              builder: (context, snapshot) => cell(snapshot.data),
            );
          }
          if (!tileFade) return tile;
          return TileEdgeFade(
            key: ValueKey(item.id),
            index: i,
            columns: columns,
            rowExtent: rowExtent,
            viewportHeight: constraints.maxHeight,
            child: tile,
          );
        }

        if (embedded) {
          // The outer scroller owns laziness and scrolling. A viewport for each
          // row adds scroll positions, clipping and layout work without benefit.
          return Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              for (var i = 0; i < columns; i++)
                Expanded(
                  child: i < items.length
                      ? buildTile(context, i)
                      : const SizedBox.shrink(),
                ),
            ],
          );
        }
        final grid = GridView.builder(
          gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: columns,
            mainAxisExtent: rowExtent,
          ),
          itemCount: items.length,
          itemBuilder: buildTile,
        );
        return !embedded && settings.scrollFade == ScrollFadeStyle.gradient
            ? ScrollEdgeFade(child: grid)
            : grid;
      },
    );
  }
}
