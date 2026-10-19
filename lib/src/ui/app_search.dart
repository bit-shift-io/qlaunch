import 'dart:async';

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/rust/usage.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cells.dart';
import 'package:qlaunch/src/ui/label_style.dart';

/// Which list the panel shows before the user types.
enum _Tab { frequent, recent }

/// The search screen the Home button replaces the launcher with.
///
/// It asks the core for nothing until it needs to: before the user types it
/// shows a Frequent or Recent list, and only a real query goes to the app
/// search. The core ranks everything; this widget draws the order it is given
/// and does no matching of its own. Tapping a result launches the app and
/// closes; the close button closes without launching.
class AppSearch extends StatefulWidget {
  const AppSearch({
    super.key,
    required this.state,
    required this.onDone,
    required this.onOpenSettings,
  });

  final LauncherState state;

  /// Called when the user picks a result or closes without picking one.
  final VoidCallback onDone;

  /// Opens the launcher settings, from the gear. Awaited so focus can return to
  /// the field when the settings screen is popped.
  final Future<void> Function() onOpenSettings;

  static const fieldKey = ValueKey('appSearch.field');
  static const closeKey = ValueKey('appSearch.close');
  static const gearKey = ValueKey('appSearch.gear');
  static const frequentTabKey = ValueKey('appSearch.tab.frequent');
  static const recentTabKey = ValueKey('appSearch.tab.recent');
  static const resultsKey = ValueKey('appSearch.results');

  /// How many apps each usage list asks for: about a page's worth.
  static const usageLimit = 24;

  @override
  State<AppSearch> createState() => _AppSearchState();
}

class _AppSearchState extends State<AppSearch>
    with SingleTickerProviderStateMixin {
  final _focus = FocusNode();
  late final AnimationController _slide;
  late final Animation<Offset> _offset;

  String _query = '';
  _Tab _tab = _Tab.frequent;
  Future<List<Item>>? _items;

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
    _items = _itemsFor(_query);
  }

  @override
  void dispose() {
    _focus.dispose();
    _slide.dispose();
    super.dispose();
  }

  /// What the panel shows for [query]. An empty (or whitespace-only) query is
  /// the selected usage list; anything else is the core's search.
  Future<List<Item>> _itemsFor(String query) {
    final trimmed = query.trim();
    if (trimmed.isEmpty) {
      final future = _tab == _Tab.frequent
          ? widget.state.frequentApps(AppSearch.usageLimit)
          : widget.state.recentApps(AppSearch.usageLimit);
      return future.then(_fromUsage);
    }
    return widget.state.searchAllApps(trimmed).then(_fromHits);
  }

  List<Item> _fromHits(List<SearchHit> hits) => [
    for (var i = 0; i < hits.length; i++) _item(hits[i].app, hits[i].label, i),
  ];

  List<Item> _fromUsage(List<UsageHit> hits) => [
    for (var i = 0; i < hits.length; i++) _item(hits[i].app, hits[i].label, i),
  ];

  /// A result as something [AppCells] can draw. The id and group are never
  /// looked at; only the label, app and order matter to the cell.
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
      _items = _itemsFor(query);
    });
  }

  void _selectTab(_Tab tab) {
    if (_tab == tab) return;
    setState(() {
      _tab = tab;
      _items = _itemsFor(_query);
    });
  }

  void _launch(Item item) {
    final app = item.app;
    if (app == null) return;
    unawaited(widget.state.launchApp(app.package, app.activity));
    widget.onDone();
  }

  Future<void> _openSettings() async {
    await widget.onOpenSettings();
    // The settings screen took focus; give it back to the field so the user can
    // keep typing where they left off.
    if (mounted) _focus.requestFocus();
  }

  bool get _showTabs => _query.trim().isEmpty;

  @override
  Widget build(BuildContext context) {
    // The panel replaces the launcher, but it is drawn over it in the same
    // Stack. Its own surfaces (the box, the tabs, the grid) do not cover every
    // pixel, and a DecoratedBox does not claim a hit test, so without this an
    // opaque tap on the backdrop beside the tabs would reach the grid behind
    // and launch an app. This absorbs every pointer the panel does not itself
    // use; the field, buttons and cells are deeper and still win their taps.
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () {},
      child: LauncherFontScope(
        settings: widget.state.snapshot?.settings,
        // Read the text styles below the scope, or they would be the un-scaled
        // ones from above it.
        child: Builder(
          builder: (context) {
            final text = Theme.of(context).textTheme.bodyLarge;
            final settings = widget.state.snapshot?.settings;
            return DecoratedBox(
              // Opaque enough to make the grid behind it clearly inactive, but not a
              // solid fill: the wallpaper still reads through.
              decoration: const BoxDecoration(color: Color(0xE6000000)),
              child: SafeArea(
                child: Column(
                  children: [
                    SlideTransition(
                      position: _offset,
                      child: Row(
                        children: [
                          IconButton(
                            key: AppSearch.gearKey,
                            icon: const Icon(
                              Icons.settings,
                              color: Colors.white,
                            ),
                            onPressed: _openSettings,
                          ),
                          Expanded(
                            child: TextField(
                              key: AppSearch.fieldKey,
                              focusNode: _focus,
                              autofocus: true,
                              onChanged: _setQuery,
                              textInputAction: TextInputAction.search,
                              // The scoped theme carries the user's Font size and
                              // typeface; only the colour is the panel's own.
                              style: text?.copyWith(color: Colors.white),
                              decoration: InputDecoration(
                                hintText: 'Search apps',
                                hintStyle: text?.copyWith(
                                  color: Colors.white70,
                                ),
                                border: InputBorder.none,
                              ),
                            ),
                          ),
                          IconButton(
                            key: AppSearch.closeKey,
                            icon: const Icon(Icons.close, color: Colors.white),
                            onPressed: widget.onDone,
                          ),
                        ],
                      ),
                    ),
                    if (_showTabs)
                      _TabBar(tab: _tab, style: text, onSelect: _selectTab),
                    Expanded(
                      // A transparent Material so the result cells paint their ink
                      // splashes: the dark panel behind is a DecoratedBox, which
                      // would otherwise hide them.
                      child: Material(
                        type: MaterialType.transparency,
                        child: FutureBuilder<List<Item>>(
                          future: _items,
                          builder: (context, snapshot) {
                            if (settings == null) {
                              return const SizedBox.shrink();
                            }
                            return AppCells(
                              key: AppSearch.resultsKey,
                              state: widget.state,
                              settings: settings,
                              items: snapshot.data ?? const <Item>[],
                              onItemTap: _launch,
                            );
                          },
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}

/// The two tabs shown before typing. A plain two-item strip, not the home
/// [TabStrip]: there are no pages to navigate, just a choice of list.
class _TabBar extends StatelessWidget {
  const _TabBar({
    required this.tab,
    required this.style,
    required this.onSelect,
  });

  final _Tab tab;
  final TextStyle? style;
  final ValueChanged<_Tab> onSelect;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        _tab(_Tab.frequent, 'Frequent', AppSearch.frequentTabKey),
        const SizedBox(width: 24),
        _tab(_Tab.recent, 'Recent', AppSearch.recentTabKey),
      ],
    );
  }

  Widget _tab(_Tab value, String label, Key key) {
    final current = tab == value;
    final colour = style?.color ?? Colors.white;
    return GestureDetector(
      key: key,
      behavior: HitTestBehavior.opaque,
      onTap: () => onSelect(value),
      child: IntrinsicWidth(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              label,
              maxLines: 1,
              textAlign: TextAlign.center,
              style: style?.copyWith(
                color: current ? colour : colour.withValues(alpha: 0.6),
                fontWeight: current ? FontWeight.w600 : FontWeight.w400,
              ),
            ),
            const SizedBox(height: 2),
            if (current)
              Container(
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
      ),
    );
  }
}
