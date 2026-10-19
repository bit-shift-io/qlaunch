import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/label_style.dart';

/// The group pages as labels along the top of the home screen.
///
/// §8.2 gives the strip the job of showing which group the user is on; the
/// swipe is the navigation. A tap on a tab also switches to that group: the
/// pager can be commanded from outside through the controller, so the strip is
/// a control as well as an indicator.
///
/// The labels read the same font settings as app labels (see [labelTextStyle]).
/// When every tab fits they are shown together, centred as a group; when they
/// do not, the strip scrolls and keeps the current tab in the middle, so the
/// user always sees where they are and that there are neighbours either side.
class TabStrip extends StatefulWidget {
  const TabStrip({
    super.key,
    required this.pageIds,
    required this.groups,
    required this.currentGroupId,
    required this.settings,
    this.onGroupTap,
    this.onOverflowChanged,
  });

  /// The pages in display order, one tab each, in the order given. The `''`
  /// sentinel draws no tab; no page list at all is an empty strip, not a
  /// missing one.
  final List<String> pageIds;

  /// Where labels come from, looked up by id, so a group has one label.
  final List<Group> groups;

  /// The group the pager is showing, drawn as the current tab.
  final String currentGroupId;

  /// The font settings the tabs share with the app labels.
  final Settings settings;

  /// Called with a tab's group id when it is tapped. A null callback leaves the
  /// strip as a plain indicator, so it still cannot be tapped where nothing
  /// listens.
  final ValueChanged<String>? onGroupTap;

  /// Called when the strip starts or stops overflowing — the point at which the
  /// pages no longer all fit and a page indicator is needed.
  final ValueChanged<bool>? onOverflowChanged;

  /// The highlight under the current tab.
  static const indicatorKey = ValueKey('tabStrip.indicator');

  /// Where a tab sits. Keyed by index, not by group id: `pageIds` is not
  /// promised to be free of duplicates and duplicate keys throw.
  static Key tabKey(int index) => ValueKey('tabStrip.$index');

  @override
  State<TabStrip> createState() => _TabStripState();
}

class _TabStripState extends State<TabStrip> {
  final _scroll = ScrollController();

  /// One per tab, only so [Scrollable.ensureVisible] has something to aim at.
  List<GlobalKey> _tabKeys = const [];

  /// The last overflow value the parent was told, so it is told once per change.
  bool? _reportedOverflow;

  /// The group the strip has already been centred on, so a rebuild does not
  /// yank the strip back while the user scrolls it by hand.
  String? _centredOn;

  /// The width the strip last laid out at. A change means the strip must look
  /// at its overflow again, which a window resize (a rotation, a split screen)
  /// does not otherwise make it do.
  double? _width;

  @override
  void initState() {
    super.initState();
    _syncKeys();
    WidgetsBinding.instance.addPostFrameCallback((_) => _afterLayout());
  }

  @override
  void didUpdateWidget(TabStrip oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncKeys();
    if (oldWidget.currentGroupId != widget.currentGroupId) _centredOn = null;
    WidgetsBinding.instance.addPostFrameCallback((_) => _afterLayout());
  }

  @override
  void dispose() {
    _scroll.dispose();
    super.dispose();
  }

  void _syncKeys() {
    if (_tabKeys.length != widget.pageIds.length) {
      _tabKeys = [for (final _ in widget.pageIds) GlobalKey()];
    }
  }

  void _afterLayout() {
    if (!mounted) return;
    final overflow = _scroll.hasClients && _scroll.position.maxScrollExtent > 0;
    if (overflow != _reportedOverflow) {
      _reportedOverflow = overflow;
      widget.onOverflowChanged?.call(overflow);
    }
    if (overflow && _centredOn != widget.currentGroupId) {
      _centredOn = widget.currentGroupId;
      _centreCurrent();
    }
  }

  Future<void> _centreCurrent() async {
    final index = widget.pageIds.indexOf(widget.currentGroupId);
    if (index < 0 || index >= _tabKeys.length) return;
    final context = _tabKeys[index].currentContext;
    if (context == null) return;
    await Scrollable.ensureVisible(
      context,
      alignment: 0.5,
      duration: const Duration(milliseconds: 200),
      curve: Curves.easeInOut,
    );
  }

  String _label(String groupId) {
    for (final group in widget.groups) {
      if (group.id == groupId) return group.label;
    }
    // Both come from the same snapshot, so a page whose group is missing should
    // not happen; the id is still better than a blank the user cannot read.
    return groupId;
  }

  @override
  Widget build(BuildContext context) {
    final style = labelTextStyle(widget.settings);
    return Container(
      width: double.infinity,
      color: Colors.black.withValues(alpha: 0.25),
      padding: const EdgeInsets.symmetric(vertical: 6),
      // A scroll view rather than a plain row: how many groups there are and
      // how long their labels are is the user's business. The minimum width is
      // the viewport, so when the tabs fit the row is centred as a group.
      child: LayoutBuilder(
        builder: (context, constraints) {
          if (constraints.maxWidth != _width) {
            _width = constraints.maxWidth;
            WidgetsBinding.instance.addPostFrameCallback((_) => _afterLayout());
          }
          return SingleChildScrollView(
            controller: _scroll,
            scrollDirection: Axis.horizontal,
            child: ConstrainedBox(
              constraints: BoxConstraints(minWidth: constraints.maxWidth),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  for (var i = 0; i < widget.pageIds.length; i++)
                    _tab(style, i, widget.pageIds[i]),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  Widget _tab(TextStyle style, int index, String groupId) {
    final current = groupId == widget.currentGroupId;
    final colour = style.color ?? Colors.white;
    // IntrinsicWidth lets the underline stretch to the label's own width, so
    // it reads as belonging to that word rather than a fixed stub.
    final column = IntrinsicWidth(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            _label(groupId),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            textAlign: TextAlign.center,
            style: style.copyWith(
              // Dim the ones the user is not on, so the current tab stands out
              // without throwing away the chosen colour.
              color: current ? colour : colour.withValues(alpha: 0.6),
              fontWeight: current ? FontWeight.w600 : FontWeight.w400,
            ),
          ),
          const SizedBox(height: 2),
          // An empty box of the same height keeps every tab the same size,
          // so the labels stay on one baseline.
          if (current)
            Container(
              key: TabStrip.indicatorKey,
              height: 2,
              decoration: BoxDecoration(
                color: colour,
                borderRadius: BorderRadius.circular(1),
              ),
            )
          else
            const SizedBox(height: 2),
        ],
      ),
    );
    final tab = KeyedSubtree(
      key: TabStrip.tabKey(index),
      child: Padding(
        key: _tabKeys[index],
        padding: const EdgeInsets.symmetric(horizontal: 10),
        child: column,
      ),
    );
    final onTap = widget.onGroupTap;
    if (onTap == null) return tab;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () => onTap(groupId),
      child: tab,
    );
  }
}
