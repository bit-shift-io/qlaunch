import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_grid.dart';
import 'package:qlaunch/src/ui/app_search.dart';
import 'package:qlaunch/src/ui/background_layer.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/settings_screen.dart';
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
  late int _seenHomeRequests;
  String? _shownError;

  /// Whether the Home-button search panel is over the launcher.
  bool _searching = false;

  @override
  void initState() {
    super.initState();
    _controller = GroupPagerController(pageIds: widget.state.pageIds);
    // Open on the Home group, or the first page if no group holds it (§8.2):
    // the old app always opened on its home screen, and a launcher that opens
    // somewhere else is a surprise.
    _goToHome();
    _currentGroupId = _controller.currentGroupId(_controller.currentPage);
    _seenHomeRequests = widget.state.homeRequests;
    widget.state.addListener(_onStateChanged);
  }

  /// Moves the controller to the group the Home button would go to: the group
  /// holding the Home role, or the first page if none does. Returns whether the
  /// pager moved. `false` also covers the Home group not being one of the
  /// pages, in which case the pager stays where it is.
  bool _goToHome() {
    final pages = widget.state.pageIds;
    final target =
        widget.state.homeGroup?.id ?? (pages.isEmpty ? null : pages.first);
    if (target == null) return false;
    if (_controller.currentGroupId(_controller.currentPage) == target) {
      return false;
    }
    return _controller.goToGroup(target);
  }

  @override
  void dispose() {
    widget.state.removeListener(_onStateChanged);
    super.dispose();
  }

  void _onStateChanged() {
    _syncPages();
    _handleHomePress();
    _reportNewError();
  }

  /// What the system Home button does, in order: leave a pushed screen; close
  /// the search; go to the home group; and, already home, open the search.
  ///
  /// §8.2 defines the first three and leaves the last as a no-op; opening the
  /// search there is at the user's request, and pressing Home again closes it.
  void _handleHomePress() {
    if (widget.state.homeRequests == _seenHomeRequests) return;
    _seenHomeRequests = widget.state.homeRequests;

    // A pushed screen (the settings) is what the user is looking at: Home
    // returns to the launcher rather than changing something underneath it.
    final route = ModalRoute.of(context);
    if (route != null && !route.isCurrent) {
      Navigator.of(context).popUntil((r) => r.isFirst);
      return;
    }

    if (_searching) {
      _closeSearch();
      return;
    }

    if (_goToHome()) {
      setState(
        () => _currentGroupId = _controller.currentGroupId(
          _controller.currentPage,
        ),
      );
      return;
    }

    _openSearch();
  }

  void _openSearch() => setState(() => _searching = true);

  void _closeSearch() => setState(() => _searching = false);

  /// Opens the launcher settings, the destination §8.2 gives a long press on
  /// blank space. Pushed directly rather than by route name: this screen holds
  /// the state the settings need.
  void _openSettings() {
    unawaited(
      Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (context) => SettingsScreen(state: widget.state),
        ),
      ),
    );
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
                      // A long press on blank space opens the settings (§8.2).
                      // Opaque so a press in a gap between cells still reaches
                      // this, while a cell claims its own press (AppCell does),
                      // so a press on an item never opens the settings.
                      child: GestureDetector(
                        behavior: HitTestBehavior.opaque,
                        onLongPress: _openSettings,
                        child: GroupPager(
                          controller: _controller,
                          onGroupChanged: _onGroupChanged,
                          // Each page is built with the group the controller
                          // puts at that index, so nothing here converts an
                          // index itself.
                          gridFor: (context, groupId) =>
                              AppGrid(state: widget.state, groupId: groupId),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              if (_searching)
                Positioned.fill(
                  child: AppSearch(
                    state: widget.state,
                    onDone: _closeSearch,
                  ),
                ),
            ],
          ),
        );
      },
    );
  }
}
