import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/group_apps_screen.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() {
    fake = FakeBackend(
      snapshot: snapshotOf(
        rootGroupId: 'root',
        pageIds: ['apps', 'games'],
        groups: [
          Group(id: 'root', label: 'root', order: 0, hidden: false),
          makeGroup('apps', role: GroupRole.apps, order: 0),
          makeGroup('games', role: GroupRole.games, order: 1),
        ],
        items: [
          makeItem(
            'a1',
            'apps',
            app: AppRef(package: 'p.a1', activity: 'Main'),
          ),
          makeItem(
            'g1',
            'games',
            app: AppRef(package: 'p.g1', activity: 'Main'),
          ),
          makeItem(
            'dead',
            'apps',
            app: AppRef(package: 'p.dead', activity: 'Main'),
            uninstalled: true,
          ),
          makeItem('cut', 'apps'),
        ],
      ),
    );
    state = LauncherState(fake);
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pump(WidgetTester tester, String groupId) async {
    await state.load();
    await tester.pumpWidget(
      MaterialApp(
        home: GroupAppsScreen(state: state, groupId: groupId),
      ),
    );
    await tester.pump();
  }

  testWidgets('apps already in the group are checked', (tester) async {
    await pump(tester, 'games');

    expect(
      tester
          .widget<CheckboxListTile>(find.byKey(GroupAppsScreen.tileKey('g1')))
          .value,
      isTrue,
    );
    expect(
      tester
          .widget<CheckboxListTile>(find.byKey(GroupAppsScreen.tileKey('a1')))
          .value,
      isFalse,
    );
  });

  testWidgets('ticking an app moves it into the group', (tester) async {
    await pump(tester, 'games');

    await tester.tap(find.byKey(GroupAppsScreen.tileKey('a1')));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('moveItem(a1,games,1)'));
  });

  testWidgets('unticking an app moves it to the Apps group', (tester) async {
    await pump(tester, 'games');

    await tester.tap(find.byKey(GroupAppsScreen.tileKey('g1')));
    await tester.pumpAndSettle();

    // Apps holds a1, dead and cut (3 items), so the app lands at index 3.
    expect(fake.calls, contains('moveItem(g1,apps,3)'));
  });

  testWidgets('unticking is disabled in the Apps group', (tester) async {
    await pump(tester, 'apps');

    expect(
      tester
          .widget<CheckboxListTile>(find.byKey(GroupAppsScreen.tileKey('a1')))
          .onChanged,
      isNull,
    );
  });

  testWidgets('uninstalled and app-less items are not listed', (tester) async {
    await pump(tester, 'apps');

    expect(find.byKey(GroupAppsScreen.tileKey('dead')), findsNothing);
    expect(find.byKey(GroupAppsScreen.tileKey('cut')), findsNothing);
  });
}
