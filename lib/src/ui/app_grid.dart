import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cells.dart';

/// The items of one group as a grid of app cells.
///
/// Items are drawn in the order [LauncherState.itemsIn] returns them, which is
/// already the display order. The grid listens to [state] itself, so it follows
/// settings and item changes whoever builds it; the cells themselves are drawn
/// by [AppCells].
class AppGrid extends StatelessWidget {
  const AppGrid({
    super.key,
    required this.state,
    required this.groupId,
    this.onItemTap,
    this.onItemLongPress,
    this.onItemPointerDown,
  });

  final LauncherState state;

  /// The group to show. `''` means there are no pages, and shows nothing.
  final String groupId;

  /// Replaces the default tap, which launches the item's app. Called only for
  /// items that can launch: an item with no app, or whose app is uninstalled,
  /// is never passed here.
  final ValueChanged<Item>? onItemTap;

  /// Called for any item. With none given, a long-press does nothing.
  final ValueChanged<Item>? onItemLongPress;

  /// Reports the pointer that lands on a cell, so the page can tell a blank
  /// tap from one that belongs to an item.
  final ValueChanged<int>? onItemPointerDown;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: state,
      builder: (context, _) {
        final settings = state.snapshot?.settings;
        if (settings == null) return const SizedBox.shrink();
        return AppCells(
          state: state,
          settings: settings,
          items: state.itemsIn(groupId),
          onItemTap: onItemTap,
          onItemLongPress: onItemLongPress,
          onItemPointerDown: onItemPointerDown,
        );
      },
    );
  }
}
