import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cell.dart';
import 'package:qlaunch/src/ui/folder_dialog.dart';
import 'package:qlaunch/src/ui/folder_popup.dart';
import 'package:qlaunch/src/ui/grid_geometry.dart';
import 'package:qlaunch/src/ui/raster_cache_identity.dart';
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
    this.onPullDownNotifications,
    this.embedded = false,
    this.columns,
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

  /// Called when the grid is already at its top and the user pulls down far
  /// enough. Null disables the gesture.
  final VoidCallback? onPullDownNotifications;

  /// A non-scrolling row inside the continuous group scroller.
  final bool embedded;

  /// Overrides the column count from [settings], so a caller can ask for a
  /// narrower grid while keeping the same cell geometry.
  final int? columns;

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
      (this.columns ??
              (portrait
                  ? settings.gridColumnsPortrait
                  : settings.gridColumnsLandscape))
          .toInt(),
    );
    // The bridge takes physical pixels; the setting is in dp.
    final sizePx = (settings.iconSize * MediaQuery.devicePixelRatioOf(context))
        .round();
    final labelHeight = gridLabelHeight(context, settings);

    return LayoutBuilder(
      builder: (context, constraints) {
        final cellWidth = constraints.maxWidth / columns;
        final rowExtent = embedded
            ? constraints.maxHeight
            : gridRowExtent(
                context,
                settings,
                cellWidth,
                measuredLabelHeight: labelHeight,
              );
        final geometry = appCellGeometry(
          context,
          settings,
          BoxConstraints.tightFor(width: cellWidth, height: rowExtent),
          measuredLabelHeight: labelHeight,
        );
        final tileFade =
            !embedded && settings.scrollFade == ScrollFadeStyle.tile;
        // Only an installed app has a platform icon, and only an item with a
        // stored override has bytes to read. Everything else draws the
        // placeholder. A hidden or zero-sized icon is not worth a request.
        ({ImageProvider? cached, Future<ImageProvider?>? future}) resolve(
          Item target,
        ) {
          if (!settings.iconsVisible || sizePx <= 0) {
            return (cached: null, future: null);
          }
          final app = target.app;
          if (app != null && !target.uninstalled) {
            final cached = state.cachedIcon(app.package, app.activity, sizePx);
            return (
              cached: cached,
              future: cached == null
                  ? state.iconFor(app.package, app.activity, sizePx)
                  : null,
            );
          }
          if (target.iconOverride != null) {
            final cached = state.cachedItemIcon(
              target.id,
              target.iconOverride!,
              sizePx,
            );
            return (
              cached: cached,
              future: cached == null
                  ? state.itemIcon(
                      target.id,
                      reference: target.iconOverride!,
                      sizePx: sizePx,
                    )
                  : null,
            );
          }
          return (cached: null, future: null);
        }

        Widget buildTile(BuildContext context, int i) {
          final item = items[i];
          final folderId = item.folderGroupId;
          final isFolder = folderId != null;
          // Every folder looks the same: the generic folder glyph, never a
          // member's app icon.
          final ({ImageProvider? cached, Future<ImageProvider?>? future})
          resolved = isFolder ? (cached: null, future: null) : resolve(item);
          final cached = resolved.cached;
          final icon = resolved.future;
          Widget cell(ImageProvider? icon) => AppCell(
            item: item,
            icon: icon,
            settings: settings,
            folder: isFolder,
            geometry: geometry,
            onTap: () {
              if (folderId != null) {
                unawaited(FolderPopup.show(context, state, folderId));
              } else {
                (onItemTap ?? _launch)(item);
              }
            },
            onLongPress: folderId != null
                ? () {
                    final group = state.groupById(folderId);
                    if (group == null) return;
                    unawaited(
                      showDialog<void>(
                        context: context,
                        builder: (context) =>
                            FolderDialog.edit(state: state, folderGroup: group),
                      ),
                    );
                  }
                : (onItemLongPress == null
                      ? null
                      : () => onItemLongPress!(item)),
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
        final pullDown = onPullDownNotifications;
        final grid = GridView.builder(
          physics: pullDown == null
              ? null
              : const AlwaysScrollableScrollPhysics(
                  parent: ClampingScrollPhysics(),
                ),
          gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: columns,
            mainAxisExtent: rowExtent,
          ),
          itemCount: items.length,
          itemBuilder: buildTile,
        );
        final pullable = pullDown == null
            ? grid
            : _PullDownNotifications(onTriggered: pullDown, child: grid);
        return !embedded && settings.scrollFade == ScrollFadeStyle.gradient
            ? ScrollEdgeFade(child: pullable)
            : pullable;
      },
    );
  }
}

class _PullDownNotifications extends StatefulWidget {
  const _PullDownNotifications({
    required this.onTriggered,
    required this.child,
  });

  static const triggerExtent = 48.0;

  final VoidCallback onTriggered;
  final Widget child;

  @override
  State<_PullDownNotifications> createState() => _PullDownNotificationsState();
}

class _PullDownNotificationsState extends State<_PullDownNotifications> {
  double _pulled = 0;
  bool _triggered = false;

  bool _onNotification(ScrollNotification notification) {
    if (notification is ScrollStartNotification) {
      _pulled = 0;
      _triggered = false;
    } else if (notification is OverscrollNotification &&
        !_triggered &&
        notification.dragDetails != null &&
        notification.overscroll < 0) {
      _pulled -= notification.overscroll;
      if (_pulled >= _PullDownNotifications.triggerExtent) {
        _triggered = true;
        HapticFeedback.lightImpact();
        widget.onTriggered();
      }
    } else if (notification is ScrollEndNotification) {
      _pulled = 0;
      _triggered = false;
    }
    return false;
  }

  @override
  Widget build(BuildContext context) {
    return NotificationListener<ScrollNotification>(
      onNotification: _onNotification,
      child: widget.child,
    );
  }
}
