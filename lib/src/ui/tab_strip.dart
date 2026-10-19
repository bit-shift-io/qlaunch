import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';

/// The group pages as labels along the top of the home screen.
///
/// §8.2 gives the strip the job of showing which group the user is on; the
/// swipe is the navigation. A tap on a tab also switches to that group: the
/// pager can be commanded from outside through the controller, so the strip is
/// a control as well as an indicator.
class TabStrip extends StatelessWidget {
  const TabStrip({
    super.key,
    required this.pageIds,
    required this.groups,
    required this.currentGroupId,
    this.onGroupTap,
  });

  /// The pages in display order, one tab each, in the order given. The `''`
  /// sentinel draws no tab; no page list at all is an empty strip, not a
  /// missing one.
  final List<String> pageIds;

  /// Where labels come from, looked up by id, so a group has one label.
  final List<Group> groups;

  /// The group the pager is showing, drawn as the current tab.
  final String currentGroupId;

  /// Called with a tab's group id when it is tapped. A null callback leaves the
  /// strip as a plain indicator, so it still cannot be tapped where nothing
  /// listens.
  final ValueChanged<String>? onGroupTap;

  /// The highlight under the current tab.
  static const indicatorKey = ValueKey('tabStrip.indicator');

  /// Where a tab sits. Keyed by index, not by group id: `pageIds` is not
  /// promised to be free of duplicates and duplicate keys throw.
  static Key tabKey(int index) => ValueKey('tabStrip.$index');

  String _label(String groupId) {
    for (final group in groups) {
      if (group.id == groupId) return group.label;
    }
    // Both come from the same snapshot, so a page whose group is missing should
    // not happen; the id is still better than a blank the user cannot read.
    return groupId;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      color: Colors.black.withValues(alpha: 0.25),
      padding: const EdgeInsets.symmetric(vertical: 6),
      // A scroll view rather than a plain row: how many groups there are and
      // how long their labels are is the user's business.
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            for (var i = 0; i < pageIds.length; i++) _tab(theme, i, pageIds[i]),
          ],
        ),
      ),
    );
  }

  Widget _tab(ThemeData theme, int index, String groupId) {
    final current = groupId == currentGroupId;
    final tab = Padding(
      key: tabKey(index),
      padding: const EdgeInsets.symmetric(horizontal: 10),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            _label(groupId),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.labelLarge?.copyWith(
              color: current ? Colors.white : Colors.white60,
              fontWeight: current ? FontWeight.w600 : FontWeight.w400,
            ),
          ),
          const SizedBox(height: 2),
          // An empty box of the same height keeps every tab the same size,
          // so the labels stay on one baseline.
          if (current)
            Container(
              key: indicatorKey,
              height: 2,
              width: 16,
              decoration: BoxDecoration(
                color: Colors.white,
                borderRadius: BorderRadius.circular(1),
              ),
            )
          else
            const SizedBox(height: 2),
        ],
      ),
    );
    final onTap = onGroupTap;
    if (onTap == null) return tab;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () => onTap(groupId),
      child: tab,
    );
  }
}
