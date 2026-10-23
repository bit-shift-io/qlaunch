import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/state/icon_warmer.dart';
import 'package:qlaunch/src/ui/app_grid.dart';
import 'package:qlaunch/src/ui/app_search.dart';
import 'package:qlaunch/src/ui/app_settings_dialog.dart';
import 'package:qlaunch/src/ui/background_layer.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/group_scroller.dart';
import 'package:qlaunch/src/ui/launcher_menu.dart';
import 'package:qlaunch/src/ui/raster_cache_identity.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';
import 'package:qlaunch/src/ui/settings_screen.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

/// The home screen: the group pages, with the tab strip along the top.
///
/// It owns the [GroupPagerController], so this is the only place a page index
/// exists; everything below it is handed a group id.
class HomeScreen extends StatefulWidget {
  const HomeScreen({
    super.key,
    required this.state,
    this.iconWarmer,
    this.rasterStore,
  });

  final LauncherState state;

  /// Optional scheduler dependency; the screen owns and disposes the warmer.
  final IconWarmer? iconWarmer;

  /// Optional raster store dependency for tests; the session store is used
  /// when omitted.
  final RasterImageStore? rasterStore;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> with WidgetsBindingObserver {
  late final GroupPagerController _controller;
  late String _currentGroupId;
  late int _seenHomeRequests;
  late int _seenOverlayHomeRequests;
  late int _seenBackRequests;
  String? _shownError;
  late final IconWarmer _warmer;
  Object? _warmedSnapshot;
  int? _warmedSize;
  Set<String> _owners = const {};

  RasterImageStore get _rasterStore =>
      widget.rasterStore ?? TileRasterCache.store;
  bool _foreground = true;
  bool _routeVisible = true;
  final _scrolling = <(Axis, int)>{};

  /// Whether search replaces the main launcher's foreground.
  bool _searching = false;

  /// The pointer currently down on a cell, reported by [AppGrid]. The pager's
  /// own listener sees the same event and uses this to ignore item taps.
  int? _itemPointer;
  int? _trackedPointer;
  bool _pointerStartedOnItem = false;
  bool _pointerMoved = false;
  Offset? _pointerDownPosition;
  DateTime? _pointerDownAt;
  DateTime? _lastBlankTapAt;
  Offset? _lastBlankTapPosition;

  @override
  void initState() {
    super.initState();
    _warmer = widget.iconWarmer ?? IconWarmer();
    // Observers only see future transitions: a screen mounted while the app
    // is already backgrounded must not warm until it is resumed.
    final lifecycle = WidgetsBinding.instance.lifecycleState;
    _foreground = lifecycle == null || lifecycle == AppLifecycleState.resumed;
    WidgetsBinding.instance.addObserver(this);
    _controller = GroupPagerController(pageIds: widget.state.pageIds);
    // Open on the Home group, or the first page if no group holds it (§8.2):
    // the old app always opened on its home screen, and a launcher that opens
    // somewhere else is a surprise.
    _goToHome();
    _currentGroupId = _controller.currentGroupId(_controller.currentPage);
    _seenHomeRequests = widget.state.homeRequests;
    _seenOverlayHomeRequests = widget.state.overlayHomeRequests;
    _seenBackRequests = widget.state.backRequests;
    _syncOwners();
    widget.state.addListener(_onStateChanged);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _warmIcons(_currentGroupId);
    });
  }

  void _updateWarming() {
    final eligible =
        _foreground && _routeVisible && !_searching && _scrolling.isEmpty;
    _warmer.setEnabled(eligible);
    TileRasterCache.scheduler.setSpeculationEnabled(eligible);
  }

  /// Both caches have hard bounds. Stop speculating once either is full rather
  /// than evicting and reloading the working set for the whole pass.
  bool _hasIconRoom() {
    final images = PaintingBinding.instance.imageCache;
    return widget.state.iconCacheBytes < widget.state.iconCacheMaximumBytes &&
        images.currentSizeBytes < images.maximumSizeBytes;
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _routeVisible = ModalRoute.of(context)?.isCurrent ?? true;
    _updateWarming();
    final settings = widget.state.snapshot?.settings;
    final size =
        ((settings?.iconSize ?? 0) * MediaQuery.devicePixelRatioOf(context))
            .round();
    if (size != _warmedSize) _warmIcons(_currentGroupId);
  }

  /// Drops raster owners that no longer exist in the model. Moving an item
  /// keeps its identity, so a move alone does not invalidate its pixels.
  void _syncOwners() {
    final snapshot = widget.state.snapshot;
    final owners = <String>{
      if (snapshot != null) ...snapshot.items.map((item) => 'item:${item.id}'),
      if (snapshot != null)
        ...snapshot.groups.map((group) => 'group:${group.id}'),
      ...widget.state.pageIds.map((id) => 'group:$id'),
    };
    for (final owner in _owners.difference(owners)) {
      _rasterStore.invalidateOwner(owner);
    }
    _owners = owners;
  }

  /// Warm actual decoded images in page-priority order, bounded to one pass.
  void _warmIcons(String groupId) {
    final snapshot = widget.state.snapshot;
    if (snapshot == null) return;
    final settings = snapshot.settings;
    _warmedSnapshot = snapshot;
    final sizePx = (settings.iconSize * MediaQuery.devicePixelRatioOf(context))
        .round();
    _warmedSize = sizePx;
    _warmer.cancel();
    widget.state.retainIconSize(sizePx);
    if (!settings.iconsVisible || sizePx <= 0) return;

    final pageIds = widget.state.pageIds;
    final index = pageIds.indexOf(groupId);
    final groups = <String>{
      groupId,
      if (index >= 0 && pageIds.length > 1)
        pageIds[(index + 1) % pageIds.length],
      if (index >= 0 && pageIds.length > 2)
        pageIds[(index - 1 + pageIds.length) % pageIds.length],
      ...pageIds,
    };
    final requests = <IconWarmRequest>[];
    for (final group in groups) {
      for (final item in widget.state.itemsIn(group)) {
        final app = item.app;
        if (item.uninstalled) continue;
        if (app != null) {
          requests.add(
            IconWarmRequest(
              key: (app.package, app.activity, sizePx),
              estimatedBytes: sizePx * sizePx * 4,
              load: () =>
                  widget.state.iconFor(app.package, app.activity, sizePx),
            ),
          );
        } else if (item.iconOverride != null) {
          requests.add(
            IconWarmRequest(
              key: (item.id, item.iconOverride, sizePx),
              // The decoded image dominates for the small PNGs a shortcut
              // stores. Actual source bytes are accounted by the cache bounds.
              estimatedBytes: sizePx * sizePx * 4,
              load: () => widget.state.itemIcon(
                item.id,
                reference: item.iconOverride!,
                sizePx: sizePx,
              ),
            ),
          );
        }
      }
    }
    _updateWarming();
    final images = PaintingBinding.instance.imageCache;
    _warmer.replaceQueue(
      requests,
      maximumBytes: images.maximumSizeBytes,
      maximumEntries: images.maximumSize.clamp(0, 1000),
      hasRoom: _hasIconRoom,
    );
  }

  /// Moves the controller to the group the Home button would go to: the group
  /// holding the Home role, or the first page if none does. Returns whether the
  /// pager moved. `false` also covers the Home group not being one of the
  /// pages, in which case the pager stays where it is.
  bool _goToHome() {
    final pages = widget.state.pageIds;
    // The Home group can itself be hidden (any group can be hidden), and then
    // it is not one of the pages: fall back to the first page rather than
    // leaving the Home button with nowhere to go.
    final home = widget.state.homeGroup?.id;
    final target = (home != null && pages.contains(home))
        ? home
        : (pages.isEmpty ? null : pages.first);
    if (target == null) return false;
    if (_controller.currentGroupId(_controller.currentPage) == target) {
      return false;
    }
    return _controller.goToGroup(target);
  }

  void _step(int direction) {
    setState(() => _controller.step(direction));
  }

  /// Tapping a tab switches the pager to that group, same as a swipe would.
  void _goToGroup(String groupId) {
    if (_controller.currentGroupId(_controller.currentPage) == groupId &&
        widget.state.snapshot?.settings.continuousScroll != true) {
      return;
    }
    if (!_controller.goToGroup(groupId)) return;
    setState(
      () =>
          _currentGroupId = _controller.currentGroupId(_controller.currentPage),
    );
    _warmIcons(_currentGroupId);
  }

  @override
  void dispose() {
    _warmer.dispose();
    WidgetsBinding.instance.removeObserver(this);
    widget.state.removeListener(_onStateChanged);
    _controller.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _foreground = state == AppLifecycleState.resumed;
    _updateWarming();
    // Returning from another app always reveals the main tab, never search.
    // Inactive alone also covers the keyboard/system UI and must not close it.
    if (state == AppLifecycleState.paused && _searching) _closeSearch();
  }

  @override
  void didHaveMemoryPressure() {
    _warmer.cancel();
    TileRasterCache.scheduler.cancelSpeculative();
    _rasterStore.trimForMemoryPressure();
    widget.state.releaseIconSources();
    PaintingBinding.instance.imageCache.clear();
    // Speculation does not restart here: the next foreground activity does.
  }

  void _onStateChanged() {
    _syncPages();
    _handleHomePress();
    _handleOverlayHomePress();
    _handleBackPress();
    _reportNewError();
    // A new snapshot (after `AppsChanged`, say) can change this page's items;
    // the cache dedups, so warming again is cheap.
    if (!identical(_warmedSnapshot, widget.state.snapshot)) {
      _syncOwners();
      _warmIcons(_currentGroupId);
    }
  }

  /// Back closes pushed routes and search first, then cycles one visible tab
  /// left, wrapping indefinitely without leaving the launcher.
  ///
  /// A pushed screen (the settings, from the gear or a long press) is what the
  /// user is looking at, so Back leaves it first — returning to the panel with
  /// its query, tab and focus intact — exactly as Home does with a pushed
  /// screen. The manifest opts out of predictive back, so this is the only path
  /// the button takes.
  void _handleBackPress() {
    if (widget.state.backRequests == _seenBackRequests) return;
    _seenBackRequests = widget.state.backRequests;

    final route = ModalRoute.of(context);
    if (route != null && !route.isCurrent) {
      Navigator.of(context).pop();
      return;
    }

    if (_searching) {
      _closeSearch();
      return;
    }
    _step(-1);
  }

  /// What the system Home button does, in order: leave a pushed screen; close
  /// the search; go to the home group; and, already home, open the search.
  ///
  /// §8.2 defines the first three and leaves the last as a no-op; opening the
  /// search there is at the user's request, and pressing Home again closes it.
  void _handleHomePress() {
    if (widget.state.homeRequests == _seenHomeRequests) return;
    _seenHomeRequests = widget.state.homeRequests;
    _homeAction(openSearch: true);
  }

  /// Home while the notification shade (or another system overlay) was open.
  /// That press dismisses the overlay, so it returns to the launcher without
  /// advancing to the search: a Home press opens search only when the user was
  /// already looking at the home screen with nothing else in front of it.
  void _handleOverlayHomePress() {
    if (widget.state.overlayHomeRequests == _seenOverlayHomeRequests) return;
    _seenOverlayHomeRequests = widget.state.overlayHomeRequests;
    _homeAction(openSearch: false);
  }

  void _homeAction({required bool openSearch}) {
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

    if (openSearch) _openSearch();
  }

  void _openSearch() {
    setState(() => _searching = true);
    _updateWarming();
  }

  void _closeSearch() {
    setState(() => _searching = false);
    _updateWarming();
  }

  /// Opens the launcher settings, the destination §8.2 gives a long press on
  /// blank space. Pushed directly rather than by route name: this screen holds
  /// the state the settings need.
  ///
  /// Returns when the settings screen is popped, so the search gear can take
  /// focus back to its field.
  Future<void> _openSettings() => Navigator.of(context).push(
    MaterialPageRoute<void>(
      builder: (context) => SettingsScreen(state: widget.state),
    ),
  );

  void _openLauncherMenu() {
    unawaited(LauncherMenu.show(context, widget.state, _currentGroupId));
  }

  /// Tracks a pointer from down to up without joining the gesture arena, so a
  /// double tap on blank space can lock the screen without delaying taps on
  /// the cells. [AppGrid] reports the pointer when it lands on an item.
  void _onItemPointerDown(int pointer) => _itemPointer = pointer;

  void _onPointerDown(PointerDownEvent event) {
    _trackedPointer = event.pointer;
    _pointerStartedOnItem = _itemPointer == event.pointer;
    _pointerMoved = false;
    _pointerDownPosition = event.position;
    _pointerDownAt = DateTime.now();
  }

  void _onPointerMove(PointerMoveEvent event) {
    if (event.pointer != _trackedPointer) return;
    final down = _pointerDownPosition;
    if (down != null && (event.position - down).distance > kTouchSlop) {
      _pointerMoved = true;
    }
  }

  void _onPointerUp(PointerUpEvent event) {
    if (event.pointer != _trackedPointer) return;
    final onItem = _pointerStartedOnItem;
    final moved = _pointerMoved;
    final downAt = _pointerDownAt;
    _trackedPointer = null;
    _itemPointer = null;
    _pointerDownPosition = null;
    _pointerDownAt = null;
    if (onItem || moved || downAt == null) return;
    if (DateTime.now().difference(downAt) > kDoubleTapTimeout) return;

    final lastAt = _lastBlankTapAt;
    final lastPosition = _lastBlankTapPosition;
    final isDouble =
        lastAt != null &&
        lastPosition != null &&
        DateTime.now().difference(lastAt) <= kDoubleTapTimeout &&
        (event.position - lastPosition).distance <= kDoubleTapSlop;
    if (!isDouble) {
      _lastBlankTapAt = DateTime.now();
      _lastBlankTapPosition = event.position;
      return;
    }
    _lastBlankTapAt = null;
    _lastBlankTapPosition = null;
    unawaited(widget.state.lockScreen());
  }

  /// Opens the per-app popup, the destination §8.2 gives a long press on an
  /// item: rename, move to another group, or uninstall.
  void _openAppSettings(Item item) {
    unawaited(
      showDialog<void>(
        context: context,
        builder: (context) =>
            AppSettingsDialog(state: widget.state, item: item),
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
    _warmIcons(groupId);
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
        final generation = settings == null
            ? null
            : rasterGeneration(context, settings);
        if (generation != null) {
          _rasterStore.activateGeneration(generation);
        }
        final screen = Scaffold(
          // Transparent so the window's wallpaper shows behind the grid on
          // Android (§7.7); the base layer is the solid colour elsewhere. The
          // settings screen and dialogs paint their own surfaces.
          backgroundColor: Colors.transparent,
          // No drawers: their destinations are Plan 3's (F3).
          body: Stack(
            children: [
              BackgroundLayer(alpha: (settings?.wallpaperAlpha ?? 0).toInt()),
              // Insets are respected rather than drawn under (F5):
              // `status_bar_visible` is Android chrome, and belongs to the
              // Android host.
              Offstage(
                offstage: _searching,
                child: TickerMode(
                  enabled: !_searching,
                  child: SafeArea(
                    child: Column(
                      children: [
                        if (showTabs)
                          TabStrip(
                            pageIds: _controller.pageIds,
                            groups: widget.state.groups,
                            currentGroupId: _currentGroupId,
                            settings: settings,
                            controller: _controller,
                            onGroupTap: _goToGroup,
                            onStep: _step,
                          ),
                        Expanded(
                          // A long press on blank space opens the launcher menu.
                          // Opaque so a press in a gap between cells still reaches
                          // this, while a cell claims its own press (AppCell does),
                          // so a press on an item never opens the menu. The outer
                          // listener sees every pointer but only counts taps that
                          // did not start on a cell, which makes double tap work
                          // without delaying an item's own tap.
                          child: Listener(
                            onPointerDown: _onPointerDown,
                            onPointerMove: _onPointerMove,
                            onPointerUp: _onPointerUp,
                            child: GestureDetector(
                              behavior: HitTestBehavior.opaque,
                              onLongPress: _openLauncherMenu,
                              child: NotificationListener<ScrollNotification>(
                                onNotification: (notification) {
                                  final key = (
                                    notification.metrics.axis,
                                    notification.depth,
                                  );
                                  if (notification is ScrollStartNotification) {
                                    _scrolling.add(key);
                                  }
                                  if (notification is ScrollEndNotification) {
                                    _scrolling.remove(key);
                                  }
                                  _updateWarming();
                                  return false;
                                },
                                child: settings?.continuousScroll == true
                                    ? GroupScroller(
                                        state: widget.state,
                                        controller: _controller,
                                        onGroupChanged: _onGroupChanged,
                                        onStep: _step,
                                        onItemLongPress: _openAppSettings,
                                        onItemPointerDown: _onItemPointerDown,
                                      )
                                    : GroupPager(
                                        controller: _controller,
                                        transitionDuration: Duration(
                                          milliseconds:
                                              settings?.pageTransitionMs
                                                  .toInt() ??
                                              300,
                                        ),
                                        columns:
                                            (MediaQuery.orientationOf(
                                                          context,
                                                        ) ==
                                                        Orientation.portrait
                                                    ? settings
                                                          ?.gridColumnsPortrait
                                                    : settings
                                                          ?.gridColumnsLandscape)
                                                ?.toInt() ??
                                            4,
                                        onGroupChanged: _onGroupChanged,
                                        // Each page is built with the group the controller
                                        // puts at that index, so nothing here converts an
                                        // index itself.
                                        gridFor: (context, groupId) => AppGrid(
                                          state: widget.state,
                                          groupId: groupId,
                                          onItemLongPress: _openAppSettings,
                                          onItemPointerDown: _onItemPointerDown,
                                          onPullDownNotifications:
                                              defaultTargetPlatform ==
                                                  TargetPlatform.android
                                              ? () => unawaited(
                                                  widget.state
                                                      .openNotifications(),
                                                )
                                              : null,
                                        ),
                                      ),
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
              if (_searching)
                Positioned.fill(
                  child: AppSearch(
                    state: widget.state,
                    onDone: _closeSearch,
                    onOpenSettings: _openSettings,
                  ),
                ),
            ],
          ),
        );
        return generation == null
            ? screen
            : RasterCacheScope(
                store: _rasterStore,
                generation: generation,
                child: screen,
              );
      },
    );
  }
}
