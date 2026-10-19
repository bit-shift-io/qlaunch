import 'dart:async';
import 'dart:math' as math;
import 'dart:typed_data';

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
class AppCells extends StatefulWidget {
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

  @override
  State<AppCells> createState() => _AppCellsState();
}

typedef _IconKey = ({String package, String activity, int sizePx});

class _AppCellsState extends State<AppCells> {
  // The Future is cached rather than the bytes, so a rebuild does not ask again
  // and two cells for one app share a single call. The physical size is part of
  // the key, so a different pixel ratio asks again.
  //
  // Never invalidated: an app whose icon changes on disk keeps its old icon
  // until the app restarts. That is accepted.
  final Map<_IconKey, Future<Uint8List?>> _icons = {};

  Future<Uint8List?> _iconFor(String package, String activity, int sizePx) {
    return _icons.putIfAbsent((
      package: package,
      activity: activity,
      sizePx: sizePx,
    ), () => widget.state.iconFor(package, activity, sizePx));
  }

  void _launch(Item item) {
    final app = item.app;
    if (app == null) return;
    unawaited(widget.state.launchApp(app.package, app.activity));
  }

  @override
  Widget build(BuildContext context) {
    final settings = widget.settings;
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
    final items = widget.items;

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
        Widget cell(Uint8List? icon) => AppCell(
          item: item,
          icon: icon,
          settings: settings,
          onTap: () => (widget.onItemTap ?? _launch)(item),
          onLongPress: widget.onItemLongPress == null
              ? null
              : () => widget.onItemLongPress!(item),
        );
        if (!wantsIcon) {
          return KeyedSubtree(key: ValueKey(item.id), child: cell(null));
        }
        return FutureBuilder<Uint8List?>(
          key: ValueKey(item.id),
          future: _iconFor(app.package, app.activity, sizePx),
          builder: (context, snapshot) => cell(snapshot.data),
        );
      },
    );
  }
}
