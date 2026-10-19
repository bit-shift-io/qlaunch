import 'dart:async';

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/state/launcher_state.dart';

/// The search panel the Home button drops over the launcher.
///
/// It asks the core for every app as soon as it opens, and again on every
/// keystroke. The core ranks the hits; this widget draws them in the order it
/// is given, and does no matching of its own. Tapping a hit launches the app
/// and closes; the close button closes without launching.
class AppSearch extends StatefulWidget {
  const AppSearch({super.key, required this.state, required this.onDone});

  final LauncherState state;

  /// Called when the user picks a hit or closes without picking one.
  final VoidCallback onDone;

  static const fieldKey = ValueKey('appSearch.field');
  static const closeKey = ValueKey('appSearch.close');

  @override
  State<AppSearch> createState() => _AppSearchState();
}

class _AppSearchState extends State<AppSearch> {
  late Future<List<SearchHit>> _hits;

  @override
  void initState() {
    super.initState();
    // An empty query is every app, so opening the panel shows the full list.
    _hits = widget.state.searchAllApps('');
  }

  void _search(String query) {
    // Replacing the future is enough: [FutureBuilder] drops the result of a
    // future it is no longer watching, so an out-of-order reply cannot
    // overwrite a newer one.
    final hits = widget.state.searchAllApps(query);
    setState(() {
      _hits = hits;
    });
  }

  void _launch(SearchHit hit) {
    unawaited(widget.state.launchApp(hit.app.package, hit.app.activity));
    widget.onDone();
  }

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      // Opaque enough to make the grid behind it clearly inactive, but not a
      // solid fill: the wallpaper still reads through.
      decoration: const BoxDecoration(color: Color(0xE6000000)),
      child: SafeArea(
        child: Column(
          children: [
            Row(
              children: [
                const SizedBox(width: 16),
                Expanded(
                  child: TextField(
                    key: AppSearch.fieldKey,
                    autofocus: true,
                    onChanged: _search,
                    textInputAction: TextInputAction.search,
                    style: const TextStyle(color: Colors.white),
                    decoration: const InputDecoration(
                      hintText: 'Search apps',
                      hintStyle: TextStyle(color: Colors.white70),
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
            Expanded(
              // A transparent Material so the result tiles paint their ink
              // splashes: the dark panel behind is a DecoratedBox, which would
              // otherwise hide them.
              child: Material(
                type: MaterialType.transparency,
                child: FutureBuilder<List<SearchHit>>(
                  future: _hits,
                  builder: (context, snapshot) {
                    final hits = snapshot.data ?? const <SearchHit>[];
                    return ListView.builder(
                      itemCount: hits.length,
                      itemBuilder: (context, i) {
                        final hit = hits[i];
                        return ListTile(
                          title: Text(hit.label),
                          textColor: Colors.white,
                          onTap: () => _launch(hit),
                        );
                      },
                    );
                  },
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
