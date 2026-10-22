import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cells.dart';
import 'package:qlaunch/src/ui/grid_geometry.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/label_style.dart';
import 'package:qlaunch/src/ui/scroll_edge_fade.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;

/// A lazy, bidirectional list of repeating group headings and app rows.
/// Two slivers grow away from the first group's heading, so neither direction
/// needs an artificial end or a jump back to the start of the list.
class GroupScroller extends StatefulWidget {
  const GroupScroller({
    super.key,
    required this.state,
    required this.controller,
    required this.onGroupChanged,
    required this.onStep,
    this.onItemLongPress,
    this.onItemPointerDown,
  });

  final LauncherState state;
  final GroupPagerController controller;
  final ValueChanged<String> onGroupChanged;
  final ValueChanged<int> onStep;
  final ValueChanged<Item>? onItemLongPress;
  final ValueChanged<int>? onItemPointerDown;

  @override
  State<GroupScroller> createState() => _GroupScrollerState();
}

class _GroupScrollerState extends State<GroupScroller> {
  static const _center = ValueKey('groupScroller.center');
  ScrollController? _scroll;
  _Cycle? _cycle;
  Object? _snapshot;
  int? _columns;
  Widget? _list;
  int? _revision;
  int? _command;
  bool _reportQueued = false;
  double _horizontalDistance = 0;

  @override
  void dispose() {
    _scroll?.dispose();
    super.dispose();
  }

  void _onScroll() {
    if (_reportQueued) return;
    _reportQueued = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _reportQueued = false;
      if (!mounted || _cycle == null || !_scroll!.hasClients) return;
      final cycle = _cycle!;
      final offset = _scroll!.offset;
      final lap = (offset / cycle.extent).floor();
      final within = offset - lap * cycle.extent;
      var group = cycle.groupOffsets.length - 1;
      while (group > 0 && cycle.groupOffsets[group] > within + 0.5) {
        group--;
      }
      final controller = widget.controller;
      final page = controller.initialPage + lap * cycle.ids.length + group;
      controller.onPageChanged(page);
      controller.position.value = page.toDouble();
      widget.onGroupChanged(controller.groupIdAt(page));
    });
  }

  void _moveTo(int page, {required bool animate}) {
    if (!mounted || _cycle == null || !_scroll!.hasClients) return;
    final cycle = _cycle!;
    final relative = page - widget.controller.initialPage;
    final group = relative % cycle.ids.length;
    final lap = (relative / cycle.ids.length).floor();
    final offset = lap * cycle.extent + cycle.groupOffsets[group];
    if (animate && !MediaQuery.disableAnimationsOf(context)) {
      _scroll!.animateTo(
        offset,
        duration: const Duration(milliseconds: 250),
        curve: Curves.easeOutCubic,
      );
    } else {
      _scroll!.jumpTo(offset);
    }
    _onScroll();
  }

  @override
  Widget build(BuildContext context) {
    final settings = widget.state.snapshot?.settings;
    final ids = widget.controller.pageIds;
    if (settings == null || ids.isEmpty) return const SizedBox.expand();
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = math.max(
          1,
          (MediaQuery.orientationOf(context) == Orientation.portrait
                  ? settings.gridColumnsPortrait
                  : settings.gridColumnsLandscape)
              .toInt(),
        );
        final rowExtent = gridRowExtent(
          context,
          settings,
          constraints.maxWidth / columns,
        );
        final headingExtent = math.max(
          48.0,
          MediaQuery.textScalerOf(context)
                      .scale(settings.labelSize.toDouble()) *
                  1.4 +
              20,
        );
        final dataChanged =
            !identical(_snapshot, widget.state.snapshot) ||
            _columns != columns ||
            _cycle?.rowExtent != rowExtent ||
            _cycle?.headingExtent != headingExtent ||
            _revision != widget.controller.revision;
        final cycle = dataChanged
            ? _Cycle(widget.state, ids, columns, rowExtent, headingExtent)
            : _cycle!;
        _snapshot = widget.state.snapshot;
        _columns = columns;
        if (dataChanged) _list = null;
        final geometryChanged = _cycle != null && !_cycle!.sameGeometry(cycle);
        _cycle = cycle;
        final controller = widget.controller;
        final listChanged = _revision != controller.revision;
        final commanded = _command != controller.command;
        _revision = controller.revision;
        _command = controller.command;
        if (_scroll == null) {
          final group =
              (controller.currentPage - controller.initialPage) % ids.length;
          _scroll = ScrollController(
            initialScrollOffset: cycle.groupOffsets[group],
          )..addListener(_onScroll);
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (mounted) _onScroll();
          });
        } else if (geometryChanged || listChanged || commanded) {
          final target = controller.currentPage;
          WidgetsBinding.instance.addPostFrameCallback((_) {
            _moveTo(
              target,
              animate: commanded && !listChanged && !geometryChanged,
            );
          });
        }

        Widget row(int index) {
          final entry = cycle.rows[index % cycle.rows.length];
          if (entry.items == null) {
            final label =
                widget.state.groups
                    .where((g) => g.id == entry.groupId)
                    .firstOrNull
                    ?.label ??
                entry.groupId;
            return SizedBox(
              height: headingExtent,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Row(
                  children: [
                    Expanded(child: Divider(color: Colors.white24)),
                    Expanded(
                      flex: 2,
                      child: Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 12),
                        child: Text(
                          label,
                          textAlign: TextAlign.center,
                          style: labelTextStyle(settings),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ),
                    Expanded(child: Divider(color: Colors.white24)),
                  ],
                ),
              ),
            );
          }
          return SizedBox(
            height: rowExtent,
            child: AppCells(
              state: widget.state,
              settings: settings,
              items: entry.items!,
              embedded: true,
              onItemLongPress: widget.onItemLongPress,
              onItemPointerDown: widget.onItemPointerDown,
            ),
          );
        }

        // Changing the active tab or issuing a scroll command must not rebuild
        // every visible row. Only new data or geometry replaces this subtree.
        final list = _list ??= CustomScrollView(
          key: const ValueKey('groupScroller'),
          controller: _scroll,
          center: _center,
          physics: const ClampingScrollPhysics(),
          slivers: [
            SliverList.builder(
              itemBuilder: (context, index) => row(-index - 1),
            ),
            SliverList.builder(
              key: _center,
              itemBuilder: (context, index) => row(index),
            ),
          ],
        );
        return GestureDetector(
          behavior: HitTestBehavior.opaque,
          onHorizontalDragStart: (_) => _horizontalDistance = 0,
          onHorizontalDragUpdate: (details) =>
              _horizontalDistance += details.delta.dx,
          onHorizontalDragEnd: (details) {
            final velocity = details.primaryVelocity ?? 0;
            if (_horizontalDistance.abs() < 40 && velocity.abs() < 300) return;
            final direction = velocity.abs() >= 300
                ? velocity
                : _horizontalDistance;
            widget.onStep(direction < 0 ? 1 : -1);
          },
          child: settings.scrollFade == ScrollFadeStyle.none
              ? list
              : ScrollEdgeFade(child: list),
        );
      },
    );
  }
}

class _SectionRow {
  _SectionRow(this.groupId, this.items);
  final String groupId;
  final List<Item>? items;
}

class _Cycle {
  _Cycle(
    LauncherState state,
    this.ids,
    int columns,
    this.rowExtent,
    this.headingExtent,
  ) {
    for (final id in ids) {
      groupOffsets.add(extent);
      rows.add(_SectionRow(id, null));
      extent += headingExtent;
      final items = state.itemsIn(id);
      for (var start = 0; start < items.length; start += columns) {
        rows.add(
          _SectionRow(
            id,
            items.sublist(start, math.min(start + columns, items.length)),
          ),
        );
        extent += rowExtent;
      }
    }
  }

  final List<String> ids;
  final double rowExtent;
  final double headingExtent;
  final rows = <_SectionRow>[];
  final groupOffsets = <double>[];
  double extent = 0;

  bool sameGeometry(_Cycle other) =>
      rowExtent == other.rowExtent &&
      headingExtent == other.headingExtent &&
      ids.join('\u0000') == other.ids.join('\u0000') &&
      groupOffsets.join(',') == other.groupOffsets.join(',') &&
      extent == other.extent;
}
