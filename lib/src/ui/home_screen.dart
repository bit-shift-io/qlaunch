import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_grid.dart';
import 'package:qlaunch/src/ui/background_layer.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

/// The home screen: the group pages, with the tab strip along the top.
///
/// It owns the [GroupPagerController], so this is the only place a page index
/// exists; everything below it is handed a group id.
class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key, required this.state});

  final LauncherState state;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  late final GroupPagerController _controller;
  late String _currentGroupId;
  String? _shownError;

  @override
  void initState() {
    super.initState();
    _controller = GroupPagerController(pageIds: widget.state.pageIds);
    _currentGroupId = _controller.currentGroupId(_controller.currentPage);
    widget.state.addListener(_onStateChanged);
  }

  @override
  void dispose() {
    widget.state.removeListener(_onStateChanged);
    super.dispose();
  }

  void _onStateChanged() {
    _syncPages();
    _reportNewError();
  }

  /// Shows a failure the user would otherwise never see.
  ///
  /// Nothing else renders [LauncherState.error] once the launcher has started,
  /// and a tap that does nothing at all is worse than a message. Each error is
  /// shown once; clearing it (the next successful fetch) lets the same text be
  /// shown again if it comes back.
  void _reportNewError() {
    final error = widget.state.error;
    if (error == _shownError) return;
    _shownError = error;
    if (error == null || !mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(error)));
  }

  /// Keeps the controller's page list in step with the model.
  ///
  /// Assigning [GroupPagerController.pageIds] re-anchors the pager onto the
  /// canonical index of the current group, which resets a fractional scroll
  /// position and cancels a drag in progress. Assigning on every notification
  /// would therefore fight the user's finger; assign only on a real change.
  void _syncPages() {
    final pageIds = widget.state.pageIds;
    if (listEquals(pageIds, _controller.pageIds)) return;
    _controller.pageIds = pageIds;
    // The pager reports the group it ends up on after the next frame. Taking it
    // from the controller now stops the strip pointing at the old group for
    // that frame; the pager reports the same group, which changes nothing.
    _onGroupChanged(_controller.currentGroupId(_controller.currentPage));
  }

  void _onGroupChanged(String groupId) {
    if (!mounted || groupId == _currentGroupId) return;
    setState(() => _currentGroupId = groupId);
  }

  bool _tabsVisible(TabVisibility visibility, Orientation orientation) =>
      switch (visibility) {
        TabVisibility.always => true,
        TabVisibility.never => false,
        TabVisibility.landscapeOnly => orientation == Orientation.landscape,
        TabVisibility.portraitOnly => orientation == Orientation.portrait,
      };

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.state,
      builder: (context, _) {
        final settings = widget.state.snapshot?.settings;
        // The strip follows the orientation the window actually has;
        // `Settings.orientation` is a request to the OS, not a layout input.
        final showTabs =
            settings != null &&
            _tabsVisible(
              settings.tabVisibility,
              MediaQuery.orientationOf(context),
            );
        return Scaffold(
          // The wallpaper base layer is under everything, per §7.7. It carries
          // no wallpaper bytes: `wallpaper_png()` is Android's and was not part
          // of the bridge Task 14 built, so Linux gets the solid base colour.
          // TODO(plan-2): fetch the bytes and pass them here (F2).
          //
          // No drawers: their destinations are Plan 3's (F3).
          body: Stack(
            children: [
              BackgroundLayer(alpha: (settings?.wallpaperAlpha ?? 0).toInt()),
              // Insets are respected rather than drawn under (F5):
              // `status_bar_visible` is Android chrome, and belongs to the
              // Android host.
              SafeArea(
                child: Column(
                  children: [
                    if (showTabs)
                      TabStrip(
                        pageIds: _controller.pageIds,
                        groups: widget.state.groups,
                        currentGroupId: _currentGroupId,
                      ),
                    Expanded(
                      child: GroupPager(
                        controller: _controller,
                        onGroupChanged: _onGroupChanged,
                        // Each page is built with the group the controller puts
                        // at that index, so nothing here converts an index
                        // itself.
                        gridFor: (context, groupId) =>
                            AppGrid(state: widget.state, groupId: groupId),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}
