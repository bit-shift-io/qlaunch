import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/rust/usage.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cells.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/label_style.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

/// Search replaces the main foreground, sharing its wallpaper and settings.
/// Empty queries show paged usage history; real queries use the core's ranking.
class AppSearch extends StatefulWidget {
  const AppSearch({
    super.key,
    required this.state,
    required this.onDone,
    required this.onOpenSettings,
  });

  final LauncherState state;
  final VoidCallback onDone;
  final Future<void> Function() onOpenSettings;

  static const fieldKey = ValueKey('appSearch.field');
  static const closeKey = ValueKey('appSearch.close');
  static const gearKey = ValueKey('appSearch.gear');
  static const resultsKey = ValueKey('appSearch.results');

  @override
  State<AppSearch> createState() => _AppSearchState();
}

class _AppSearchState extends State<AppSearch>
    with SingleTickerProviderStateMixin {
  // These are presentation pages, not groups in the persistent launcher model.
  static const _pageIds = ['frequent', 'recent'];
  static const _groups = [
    Group(id: 'frequent', label: 'Frequent', order: 0, hidden: false, folder: false),
    Group(id: 'recent', label: 'Recent', order: 1, hidden: false, folder: false),
  ];

  final _focus = FocusNode();
  final _pager = GroupPagerController(pageIds: _pageIds);
  late final AnimationController _slide;
  late final Animation<Offset> _offset;
  String _query = '';
  String _historyPage = 'frequent';
  int _columns = 0;
  late Future<List<Item>> _frequent;
  late Future<List<Item>> _recent;
  Future<List<Item>>? _results;

  @override
  void initState() {
    super.initState();
    _slide = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 200),
    );
    _offset = Tween(
      begin: const Offset(0, -1),
      end: Offset.zero,
    ).animate(CurvedAnimation(parent: _slide, curve: Curves.easeOut));
    _slide.forward();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _loadHistory();
  }

  @override
  void didUpdateWidget(AppSearch oldWidget) {
    super.didUpdateWidget(oldWidget);
    _loadHistory();
  }

  void _loadHistory() {
    final settings = widget.state.snapshot?.settings;
    final portrait = MediaQuery.orientationOf(context) == Orientation.portrait;
    final columns = math.max(
      1,
      (portrait
                  ? settings?.gridColumnsPortrait
                  : settings?.gridColumnsLandscape)
              ?.toInt() ??
          4,
    );
    if (columns == _columns) return;
    _columns = columns;
    final limit = columns * 3;
    _frequent = widget.state
        .frequentApps(limit)
        .then((hits) => _fromUsage(hits.take(limit)));
    _recent = widget.state
        .recentApps(limit)
        .then((hits) => _fromUsage(hits.take(limit)));
  }

  @override
  void dispose() {
    _focus.dispose();
    _pager.dispose();
    _slide.dispose();
    super.dispose();
  }

  List<Item> _fromHits(List<SearchHit> hits) => [
    for (var i = 0; i < hits.length; i++) _item(hits[i].app, hits[i].label, i),
  ];

  List<Item> _fromUsage(Iterable<UsageHit> hits) => [
    for (final (i, hit) in hits.indexed) _item(hit.app, hit.label, i),
  ];

  Item _item(AppRef app, String label, int order) => Item(
    id: '${app.package}/${app.activity}',
    groupId: '',
    label: label,
    labelOverridden: false,
    app: app,
    uninstalled: false,
    order: order,
  );

  void _setQuery(String query) {
    setState(() {
      _query = query;
      _results = query.trim().isEmpty
          ? null
          : widget.state.searchAllApps(query.trim()).then(_fromHits);
    });
  }

  void _step(int direction) => setState(() => _pager.step(direction));

  void _selectPage(String page) {
    if (_pager.goToGroup(page)) setState(() {});
  }

  void _launch(Item item) {
    final app = item.app;
    if (app == null) return;
    unawaited(widget.state.launchApp(app.package, app.activity));
    widget.onDone();
  }

  Future<void> _openSettings() async {
    await widget.onOpenSettings();
    if (mounted) _focus.requestFocus();
  }

  bool get _showHistory => _query.trim().isEmpty;

  Widget _grid(Future<List<Item>>? items, {Key? key}) =>
      FutureBuilder<List<Item>>(
        key: key,
        future: items,
        builder: (context, snapshot) {
          final settings = widget.state.snapshot?.settings;
          if (settings == null) return const SizedBox.shrink();
          return AppCells(
            state: widget.state,
            settings: settings,
            items: snapshot.data ?? const [],
            onItemTap: _launch,
          );
        },
      );

  @override
  Widget build(BuildContext context) {
    final settings = widget.state.snapshot?.settings;
    if (settings == null) return const SizedBox.shrink();
    final text = labelTextStyle(settings);
    // No scrim: HomeScreen hides the main foreground and keeps the configured
    // BackgroundLayer beneath this view, including Android's system wallpaper.
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () {},
      child: LauncherFontScope(
        settings: settings,
        child: SafeArea(
          child: Column(
            children: [
              SlideTransition(
                position: _offset,
                child: Row(
                  children: [
                    IconButton(
                      key: AppSearch.gearKey,
                      icon: Icon(Icons.settings, color: text.color),
                      onPressed: _openSettings,
                    ),
                    Expanded(
                      child: TextField(
                        key: AppSearch.fieldKey,
                        focusNode: _focus,
                        autofocus: true,
                        onChanged: _setQuery,
                        textInputAction: TextInputAction.search,
                        style: text,
                        decoration: InputDecoration(
                          hintText: 'Search apps',
                          hintStyle: text.copyWith(
                            color: text.color?.withValues(alpha: 0.7),
                          ),
                          border: InputBorder.none,
                        ),
                      ),
                    ),
                    IconButton(
                      key: AppSearch.closeKey,
                      icon: Icon(Icons.close, color: text.color),
                      onPressed: widget.onDone,
                    ),
                  ],
                ),
              ),
              if (_showHistory)
                TabStrip(
                  pageIds: _pageIds,
                  groups: _groups,
                  currentGroupId: _historyPage,
                  settings: settings,
                  controller: _pager,
                  onStep: _step,
                  onGroupTap: _selectPage,
                ),
              Expanded(
                child: Material(
                  type: MaterialType.transparency,
                  child: _showHistory
                      ? GroupPager(
                          controller: _pager,
                          columns: _columns,
                          onGroupChanged: (page) =>
                              setState(() => _historyPage = page),
                          gridFor: (context, page) => _grid(
                            page == 'frequent' ? _frequent : _recent,
                            key: ValueKey(page),
                          ),
                        )
                      : _grid(_results, key: AppSearch.resultsKey),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
