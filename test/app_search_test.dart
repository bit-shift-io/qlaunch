import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_search.dart';

import 'fake_backend.dart';

void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() {
    fake = FakeBackend();
    state = LauncherState(fake);
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  SearchHit hit(String label, {String? package}) => SearchHit(
    app: AppRef(package: package ?? 'pkg.$label', activity: 'Main'),
    label: label,
    kind: MatchKind.prefix,
  );

  Future<void> pumpSearch(WidgetTester tester, {VoidCallback? onDone}) async {
    await state.load();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AppSearch(state: state, onDone: onDone ?? () {}),
        ),
      ),
    );
    await tester.pump();
  }

  testWidgets('asks the core for every app as soon as it opens', (
    tester,
  ) async {
    fake.searchAllAppsResults = (q) => [hit('Firefox'), hit('Files')];
    await pumpSearch(tester);
    await tester.pump();

    expect(fake.searchQueries, ['']);
    expect(find.text('Firefox'), findsOneWidget);
    expect(find.text('Files'), findsOneWidget);
  });

  testWidgets('typing searches again with what was typed', (tester) async {
    fake.searchAllAppsResults = (q) =>
        q == 'fir' ? [hit('Firefox')] : [hit('Files')];
    await pumpSearch(tester);
    await tester.pump();
    expect(find.text('Files'), findsOneWidget);

    await tester.enterText(find.byKey(AppSearch.fieldKey), 'fir');
    await tester.pump();
    await tester.pump();

    expect(fake.searchQueries.last, 'fir');
    expect(find.text('Firefox'), findsOneWidget);
    expect(find.text('Files'), findsNothing);
  });

  testWidgets('the core ranks the hits and the list keeps that order', (
    tester,
  ) async {
    // Whatever order the core returns is the order shown: the widget does not
    // reorder or re-rank.
    fake.searchAllAppsResults = (q) => [hit('Zed'), hit('Alpha')];
    await pumpSearch(tester);
    await tester.pump();

    expect(
      tester.getTopLeft(find.text('Zed')).dy,
      lessThan(tester.getTopLeft(find.text('Alpha')).dy),
    );
  });

  testWidgets('tapping a hit launches the app and closes', (tester) async {
    var closed = false;
    fake.searchAllAppsResults = (q) => [
      hit('Firefox', package: 'org.mozilla.firefox'),
    ];
    await pumpSearch(tester, onDone: () => closed = true);
    await tester.pump();

    await tester.tap(find.text('Firefox'));
    await tester.pump();

    expect(fake.launched, [
      (package: 'org.mozilla.firefox', activity: 'Main'),
    ]);
    expect(closed, isTrue);
  });

  testWidgets('the close button closes without launching anything', (
    tester,
  ) async {
    var closed = false;
    fake.searchAllAppsResults = (q) => [hit('Firefox')];
    await pumpSearch(tester, onDone: () => closed = true);
    await tester.pump();

    await tester.tap(find.byKey(AppSearch.closeKey));
    await tester.pump();

    expect(closed, isTrue);
    expect(fake.launched, isEmpty);
  });

  testWidgets('no hits is an empty list, not an error', (tester) async {
    fake.searchAllAppsResults = (q) => const [];
    await pumpSearch(tester);
    await tester.pump();

    expect(find.byType(ListTile), findsNothing);
    expect(tester.takeException(), isNull);
  });
}
