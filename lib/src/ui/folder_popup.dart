import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cells.dart';
import 'package:qlaunch/src/ui/app_settings_dialog.dart';
import 'package:qlaunch/src/ui/grid_geometry.dart';
import 'package:qlaunch/src/ui/label_style.dart';

/// The floating grid of a folder's members, opened by tapping its tile.
///
/// Members are plain items, so the popup reuses [AppCells] for icons, layout
/// and launching; a long press opens the same per-item dialog the grid uses.
/// The popup is only as large as the icons it shows.
class FolderPopup extends StatelessWidget {
  const FolderPopup({
    super.key,
    required this.state,
    required this.folderGroupId,
  });

  final LauncherState state;
  final String folderGroupId;

  static const contentKey = ValueKey('folderPopup.content');
  static const emptyKey = ValueKey('folderPopup.empty');

  /// Space kept between the popup and the screen edge, besides [padding].
  static const screenMargin = 24.0;

  /// The space around the popup's grid.
  static const padding = 16.0;

  static Future<void> show(
    BuildContext context,
    LauncherState state,
    String folderGroupId,
  ) => showGeneralDialog<void>(
    context: context,
    barrierDismissible: true,
    barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
    barrierColor: Colors.black54,
    transitionDuration: const Duration(milliseconds: 120),
    pageBuilder: (context, _, _) =>
        FolderPopup(state: state, folderGroupId: folderGroupId),
    transitionBuilder: (context, animation, _, child) =>
        FadeTransition(opacity: animation, child: child),
  );

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: state,
      builder: (context, _) {
        final folder = state.groupById(folderGroupId);
        final settings = state.snapshot?.settings;
        if (folder == null || settings == null) {
          return const SizedBox.shrink();
        }
        final items = state.itemsIn(folderGroupId);
        final theme = Theme.of(context);
        return LauncherFontScope(
          settings: settings,
          child: Center(
            child: Material(
              elevation: 6,
              color:
                  theme.dialogTheme.backgroundColor ??
                  theme.colorScheme.surfaceContainerHigh,
              shape:
                  theme.dialogTheme.shape ??
                  RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(28),
                  ),
              clipBehavior: Clip.antiAlias,
              child: Padding(
                padding: const EdgeInsets.all(padding),
                child: items.isEmpty
                    ? SizedBox(
                        key: emptyKey,
                        width: 140,
                        height: 64,
                        child: Center(
                          child: Text(
                            'Empty folder',
                            textAlign: TextAlign.center,
                            style: theme.textTheme.bodyMedium,
                          ),
                        ),
                      )
                    : _grid(context, settings, items),
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _grid(BuildContext context, Settings settings, List<Item> items) {
    final portrait = MediaQuery.orientationOf(context) == Orientation.portrait;
    final configured = math.max(
      1,
      (portrait ? settings.gridColumnsPortrait : settings.gridColumnsLandscape)
          .toInt(),
    );
    final int columns = math.min(
      configured,
      math.max(1, math.sqrt(items.length).ceil()),
    );
    final screen = MediaQuery.sizeOf(context);
    final available = math.max(
      1.0,
      screen.width - screenMargin * 2 - padding * 2,
    );
    final list = settings.layout == LayoutStyle.list;
    final naturalWidth = list
        ? math.max(180.0, settings.iconSize.toDouble() + 100)
        : math.max(96.0, settings.iconSize.toDouble() + 60);
    final cellWidth = math.min(naturalWidth, available / columns);
    final rowExtent = gridRowExtent(
      context,
      settings,
      cellWidth,
      measuredLabelHeight: gridLabelHeight(context, settings),
    );
    final rows = (items.length / columns).ceil();
    final double width = columns * cellWidth;
    final double height = math.min(
      rows * rowExtent,
      screen.height * 0.7 - padding * 2,
    );

    return SizedBox(
      width: width,
      height: height,
      child: MediaQuery.removePadding(
        context: context,
        removeTop: true,
        removeBottom: true,
        removeLeft: true,
        removeRight: true,
        child: AppCells(
          key: contentKey,
          state: state,
          settings: settings,
          items: items,
          columns: columns,
          onItemLongPress: (item) => unawaited(
            showDialog<void>(
              context: context,
              builder: (context) => AppSettingsDialog(state: state, item: item),
            ),
          ),
        ),
      ),
    );
  }
}
