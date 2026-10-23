import 'package:flutter/widgets.dart';

/// The only place in the app that converts a pager index to a group.
///
/// The pager shows [virtualPageCount] virtual pages so the user can swipe
/// forever in either direction; a virtual index maps onto [pageIds] by
/// wrapping. Anything that needs to know which group an index shows, or which
/// group the user is on, asks this controller.
class GroupPagerController {
  GroupPagerController({required List<String> pageIds})
    : _pageIds = List.of(pageIds) {
    _currentPage = initialPage;
    position.value = _currentPage.toDouble();
  }

  static const int virtualPageCount = 100000;

  List<String> _pageIds;
  late int _currentPage;
  final ValueNotifier<double> position = ValueNotifier(0);
  int _revision = 0;
  int _command = 0;

  /// Changes to the group list and explicit navigation requests, respectively.
  int get revision => _revision;
  int get command => _command;

  void dispose() => position.dispose();

  void step(int direction) {
    if (_pageIds.length < 2) return;
    _currentPage += direction;
    _command++;
  }

  /// The page ids in display order. Ids are expected to be distinct: lookups by
  /// id take the first occurrence, so with `['a', 'b', 'a']` a user on the
  /// second `'a'` lands on the first after the list changes.
  ///
  /// Unmodifiable: change the list by assigning [pageIds], never by mutating
  /// what this returns.
  List<String> get pageIds => List.unmodifiable(_pageIds);

  /// Replaces the page list, keeping the user on the same *group* if it is
  /// still present. If it is gone, the pager goes to the first group of the
  /// new list. The old index is never reused: it means a different group once
  /// the list has changed.
  ///
  /// A pager already on screen picks the new [currentPage] up the next time it
  /// is rebuilt.
  set pageIds(List<String> value) {
    final remembered = _pageIds.isEmpty ? null : groupIdAt(_currentPage);
    _pageIds = List.of(value);
    final position = remembered == null ? -1 : _pageIds.indexOf(remembered);
    _currentPage = position < 0 ? initialPage : initialPage + position;
    _revision++;
  }

  /// The page the pager is showing. Starts at [initialPage] and is kept in
  /// sync by the widget reporting movement through [onPageChanged].
  int get currentPage => _currentPage;

  /// Called by the pager when the user moves to [page].
  void onPageChanged(int page) {
    _currentPage = page;
  }

  /// A large index in the middle of the virtual range whose group is
  /// `pageIds.first`, with room to scroll back from.
  ///
  /// It is a multiple of the list length, so it must be derived from the
  /// current list: a constant such as `virtualPageCount ~/ 2` lands on a
  /// different group for any length that does not divide it. With no pages
  /// there is no length to align to, so it is `virtualPageCount ~/ 2`.
  int get initialPage {
    final middle = virtualPageCount ~/ 2;
    if (_pageIds.isEmpty) return middle;
    return middle - middle % _pageIds.length;
  }

  /// The group shown at [virtualIndex]. This is the single index to group
  /// mapping; every other lookup delegates to it.
  ///
  /// Negative indices wrap to the end of the list. Returns `''` when there are
  /// no pages, so every widget must treat `''` as "there are no pages".
  String groupIdAt(int virtualIndex) {
    if (_pageIds.isEmpty) return '';
    return _pageIds[virtualIndex % _pageIds.length];
  }

  /// The canonical index for [groupId]: its position in the cycle that starts
  /// at [initialPage].
  ///
  /// Returns `-1` if the group is not in [pageIds]. That is not a valid page,
  /// and `groupIdAt(-1)` is the *last* group, so check for `-1` before using
  /// the result. This is not the nearest index to where the user is; a caller
  /// that wants that has to work it out from [currentPage] and this value.
  int virtualIndexFor(String groupId) {
    final position = _pageIds.indexOf(groupId);
    return position < 0 ? -1 : initialPage + position;
  }

  /// Moves the pager to [groupId]'s canonical index, if the group is shown.
  ///
  /// Returns `false` and changes nothing when the group is not in the list, so
  /// a caller asking for a group that no longer exists leaves the user where
  /// they were. This only moves the controller; the pager moves the next time
  /// it is rebuilt.
  bool goToGroup(String groupId) {
    final index = virtualIndexFor(groupId);
    if (index < 0) return false;
    final length = _pageIds.length;
    final forward = (index - _currentPage) % length;
    _currentPage += forward > length / 2 ? forward - length : forward;
    _command++;
    return true;
  }

  /// The group on [currentPage], the page the pager is actually showing. Same
  /// as [groupIdAt]; it exists to say what the caller means.
  String currentGroupId(int currentPage) => groupIdAt(currentPage);
}

/// A horizontally paging view of groups that wraps in both directions.
///
/// It talks to [GroupPagerController] and plain group ids only. After changing
/// `controller.pageIds`, rebuild the pager so it moves to the controller's
/// [GroupPagerController.currentPage]. It jumps to that canonical index rather
/// than the nearest copy of the group, which resets a fractional scroll
/// position and cancels a drag in progress; the group shown is the same.
class GroupPager extends StatefulWidget {
  const GroupPager({
    super.key,
    required this.controller,
    required this.gridFor,
    this.onGroupChanged,
    this.columns = 4,
  });

  final GroupPagerController controller;
  final int columns;

  /// Builds the content for a group. Receives `''` when there are no pages.
  final Widget Function(BuildContext context, String groupId) gridFor;

  /// Called when the group the pager is showing changes, whether the user
  /// swiped or the controller's page list changed under it.
  final ValueChanged<String>? onGroupChanged;

  @override
  State<GroupPager> createState() => _GroupPagerState();
}

class _GroupPagerState extends State<GroupPager> {
  late final PageController _pageController;
  late String _reportedGroup;
  bool _syncing = false;
  bool _reconcileQueued = false;
  late int _revision;
  late int _command;

  void _dragUpdate(DragUpdateDetails details) {
    if (!_pageController.hasClients) return;
    final position = _pageController.position;
    _pageController.jumpTo(
      (position.pixels - details.delta.dx).clamp(
        position.minScrollExtent,
        position.maxScrollExtent,
      ),
    );
  }

  void _settleDrag([double velocity = 0]) {
    if (!_pageController.hasClients) return;
    var page = _pageController.page ?? _targetPage.toDouble();
    if (velocity.abs() >= 300) page += velocity < 0 ? 0.5 : -0.5;
    final target = page.round().clamp(
      0,
      GroupPagerController.virtualPageCount - 1,
    );
    if (MediaQuery.disableAnimationsOf(context)) {
      _pageController.jumpToPage(target);
    } else {
      _pageController.animateToPage(
        target,
        duration: const Duration(milliseconds: 150),
        curve: Curves.easeOutCubic,
      );
    }
  }

  // With no pages every index maps to '', and 100000 blank pages would be an
  // endless scroll through nothing. One page lets the caller draw an empty
  // state. In that case this is deliberately 0 while the controller's
  // currentPage is still the anchor, so the two diverge on purpose.
  int get _targetPage =>
      widget.controller.pageIds.isEmpty ? 0 : widget.controller.currentPage;

  @override
  void initState() {
    super.initState();
    _pageController = PageController(initialPage: _targetPage);
    _revision = widget.controller._revision;
    _command = widget.controller._command;
    _pageController.addListener(_onScroll);
    _reportedGroup = widget.controller.currentGroupId(
      widget.controller.currentPage,
    );
    // The initial layout does not notify listeners, so the strip would keep
    // whatever position the controller had before it was moved onto its
    // opening group. Read the laid-out page once so the strip starts correct.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _onScroll();
    });
  }

  @override
  void didUpdateWidget(GroupPager oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_revision != widget.controller._revision) {
      _revision = widget.controller._revision;
      _command = widget.controller._command;
      // Jumping notifies the page position, and this runs while the parent is
      // building. Defer it, or a listener such as the tab strip would be
      // marked dirty during the same build.
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        _syncPage();
        _queueReconcile();
      });
    } else if (_command != widget.controller._command) {
      _command = widget.controller._command;
      final target = _targetPage;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted || !_pageController.hasClients) return;
        if (MediaQuery.disableAnimationsOf(context)) {
          _pageController.jumpToPage(target);
        } else {
          _pageController.animateToPage(
            target,
            duration: const Duration(milliseconds: 150),
            curve: Curves.easeOutCubic,
          );
        }
      });
    }
    // Reporting cannot happen here: calling the parent while it is building is
    // not safe. It also has to wait for a layout if there has not been one yet.
  }

  void _onScroll() {
    final page = _pageController.page;
    if (page != null) widget.controller.position.value = page;
  }

  @override
  void dispose() {
    _pageController.dispose();
    super.dispose();
  }

  /// Moves the view to the controller's page. Returns false if the view has
  /// not been laid out yet, so there is nothing to move.
  bool _syncPage() {
    if (!_pageController.hasClients ||
        !_pageController.position.haveDimensions) {
      return false;
    }
    if (_pageController.page?.round() != _targetPage) {
      // The jump reports a page change; the controller already holds this
      // page and _reconcile reports the group.
      _syncing = true;
      try {
        _pageController.jumpToPage(_targetPage);
      } finally {
        _syncing = false;
      }
    }
    return true;
  }

  void _queueReconcile() {
    if (_reconcileQueued) return;
    _reconcileQueued = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _reconcileQueued = false;
      _reconcile();
    });
  }

  void _reconcile() {
    if (!mounted) return;
    if (!_syncPage()) {
      _queueReconcile();
      return;
    }
    final controller = widget.controller;
    final group = controller.currentGroupId(controller.currentPage);
    if (group != _reportedGroup) _report(group);
  }

  void _report(String group) {
    _reportedGroup = group;
    widget.onGroupChanged?.call(group);
  }

  void _onPageChanged(int page) {
    if (_syncing) return;
    widget.controller.onPageChanged(page);
    _report(widget.controller.currentGroupId(page));
  }

  @override
  Widget build(BuildContext context) {
    final controller = widget.controller;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onHorizontalDragUpdate: controller.pageIds.length < 2
          ? null
          : _dragUpdate,
      onHorizontalDragEnd: controller.pageIds.length < 2
          ? null
          : (details) => _settleDrag(details.primaryVelocity ?? 0),
      onHorizontalDragCancel: controller.pageIds.length < 2
          ? null
          : _settleDrag,
      child: PageView.builder(
        controller: _pageController,
        // The custom gesture drives the page directly. Native page ballistics
        // ignore the entire viewport, swallowing nested vertical grid gestures
        // until their spring settles. Only our short release animation is needed.
        pageSnapping: false,
        itemCount: controller.pageIds.isEmpty
            ? 1
            : GroupPagerController.virtualPageCount,
        physics: const NeverScrollableScrollPhysics(),
        onPageChanged: _onPageChanged,
        itemBuilder: (context, i) => AnimatedBuilder(
          animation: _pageController,
          child: widget.gridFor(context, controller.groupIdAt(i)),
          builder: (context, child) {
            final page =
                _pageController.hasClients &&
                    _pageController.position.haveDimensions
                ? _pageController.page ?? _targetPage.toDouble()
                : _targetPage.toDouble();
            final relative = i - page;
            final width = _pageController.position.viewportDimension;
            final travel = widget.columns <= 1 ? 0.5 : 1 / widget.columns;
            return Transform.translate(
              offset: Offset(-relative * width * (1 - travel), 0),
              child: Opacity(
                // Fade a little faster than the position moves so a heavy page
                // stops compositing sooner; it still crosses the neighbour.
                opacity: (1 - relative.abs() * 1.5).clamp(0.0, 1.0),
                child: child,
              ),
            );
          },
        ),
      ),
    );
  }
}
