import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/label_style.dart';

/// Transparent, pager-linked previous/current/next group titles.
class TabStrip extends StatelessWidget {
  const TabStrip({super.key, required this.pageIds, required this.groups,
    required this.currentGroupId, required this.settings, this.controller,
    this.onGroupTap, this.onStep});

  final List<String> pageIds;
  final List<Group> groups;
  final String currentGroupId;
  final Settings settings;
  final GroupPagerController? controller;
  final ValueChanged<String>? onGroupTap;
  final ValueChanged<int>? onStep;

  static Key tabKey(int offset) => ValueKey('tabStrip.$offset');

  @override
  Widget build(BuildContext context) {
    final pager = controller;
    if (pager == null) return _titles(context, pageIds.indexOf(currentGroupId).toDouble());
    return ValueListenableBuilder<double>(valueListenable: pager.position,
      builder: (context, page, _) => _titles(context, page));
  }

  Widget _titles(BuildContext context, double page) {
    if (pageIds.isEmpty) return const SizedBox(height: 48);
    final base = page.floor();
    final fraction = page - base;
    final style = labelTextStyle(settings).copyWith(fontWeight: FontWeight.normal);
    final height = MediaQuery.textScalerOf(context).scale(settings.labelSize.toDouble()) * 1.4 + 20;
    return SizedBox(height: height < 48 ? 48 : height, child: LayoutBuilder(
      builder: (context, constraints) {
        final pitch = constraints.maxWidth / 3;
        return ClipRect(child: Stack(children: [
          for (var offset = pageIds.length == 1 ? 0 : -1;
              offset <= (pageIds.length == 1 ? 0 : 2); offset++)
            Positioned(
              left: pitch * (1 + offset - fraction), top: 0, bottom: 0, width: pitch,
              child: Semantics(selected: (offset - fraction).abs() < 0.5,
                button: true,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTap: () {
                    final delta = offset - fraction.round();
                    if (onStep != null && delta != 0) {
                      onStep!(delta.sign);
                    } else {
                      onGroupTap?.call(pageIds[(base + offset) % pageIds.length]);
                    }
                  },
                  child: Center(child: Padding(padding: const EdgeInsets.symmetric(horizontal: 8),
                    child: Text(_label(pageIds[(base + offset) % pageIds.length]),
                      key: tabKey(offset), maxLines: 1, overflow: TextOverflow.ellipsis,
                      style: style.copyWith(color: (style.color ?? Colors.white).withValues(
                        alpha: 1 - (offset - fraction).abs().clamp(0, 1) * 0.6))),
                  )),
                ),
              ),
            ),
        ]));
      },
    ));
  }

  String _label(String id) => groups.where((g) => g.id == id).firstOrNull?.label ?? id;
}
