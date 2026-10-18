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
  }

  static const int virtualPageCount = 100000;

  List<String> _pageIds;
  late int _currentPage;

  /// The page ids in display order. Unmodifiable: change the list by assigning
  /// [pageIds], never by mutating what this returns.
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

  /// The group on [currentPage], the page the pager is actually showing. Same
  /// as [groupIdAt]; it exists to say what the caller means.
  String currentGroupId(int currentPage) => groupIdAt(currentPage);
}

/// A horizontally paging view of groups that wraps in both directions.
///
/// It talks to [GroupPagerController] and plain group ids only. After changing
/// `controller.pageIds`, rebuild the pager so it moves to the controller's
/// [GroupPagerController.currentPage].
class GroupPager extends StatefulWidget {
  const GroupPager({
    super.key,
    required this.controller,
    required this.gridFor,
    this.onGroupChanged,
  });

  final GroupPagerController controller;

  /// Builds the content for a group. Receives `''` when there are no pages.
  final Widget Function(BuildContext context, String groupId) gridFor;

  /// Called with the group the user has moved to.
  final ValueChanged<String>? onGroupChanged;

  @override
  State<GroupPager> createState() => _GroupPagerState();
}

class _GroupPagerState extends State<GroupPager> {
  late final PageController _pageController;
  bool _syncing = false;

  // With no pages every index maps to '', and 100000 blank pages would be an
  // endless scroll through nothing. One page lets the caller draw an empty
  // state.
  int get _targetPage =>
      widget.controller.pageIds.isEmpty ? 0 : widget.controller.currentPage;

  @override
  void initState() {
    super.initState();
    _pageController = PageController(initialPage: _targetPage);
  }

  @override
  void didUpdateWidget(GroupPager oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_pageController.hasClients ||
        !_pageController.position.haveDimensions) {
      return;
    }
    final shown = _pageController.page?.round();
    if (shown != _targetPage) {
      // The jump reports a page change; the controller already holds this
      // page, and calling out to the parent while it is building is not safe.
      _syncing = true;
      try {
        _pageController.jumpToPage(_targetPage);
      } finally {
        _syncing = false;
      }
    }
  }

  @override
  void dispose() {
    _pageController.dispose();
    super.dispose();
  }

  void _onPageChanged(int page) {
    if (_syncing) return;
    widget.controller.onPageChanged(page);
    widget.onGroupChanged?.call(widget.controller.currentGroupId(page));
  }

  @override
  Widget build(BuildContext context) {
    final controller = widget.controller;
    return PageView.builder(
      controller: _pageController,
      itemCount: controller.pageIds.isEmpty
          ? 1
          : GroupPagerController.virtualPageCount,
      onPageChanged: _onPageChanged,
      itemBuilder: (context, i) =>
          widget.gridFor(context, controller.groupIdAt(i)),
    );
  }
}
