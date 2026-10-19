import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/rust/usage.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_search.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

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

  UsageHit usage(String label, {String? package}) => UsageHit(
    app: AppRef(package: package ?? 'pkg.$label', activity: 'Main'),
    label: label,
  );

  Future<void> pumpSearch(
    WidgetTester tester, {
    VoidCallback? onDone,
    Future<void> Function()? onOpenSettings,
  }) async {
    await state.load();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AppSearch(
            state: state,
            onDone: onDone ?? () {},
            onOpenSettings: onOpenSettings ?? () async {},
          ),
        ),
      ),
    );
    // The panel slides in from the top; settle it so the box is hittable.
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pump();
  }

  TextField field(WidgetTester tester) =>
      tester.widget<TextField>(find.byKey(AppSearch.fieldKey));

  testWidgets('an empty query shows the tabs and asks for frequent apps', (
    tester,
  ) async {
    fake.frequentAppsResults = (limit) => [usage('Firefox'), usage('Files')];
    await pumpSearch(tester);
    await tester.pump();

    expect(fake.usageQueries, ['frequent(24)']);
    expect(fake.searchQueries, isEmpty, reason: 'no app dump on open');
    expect(find.byKey(AppSearch.frequentTabKey), findsOneWidget);
    expect(find.byKey(AppSearch.recentTabKey), findsOneWidget);
    expect(find.text('Firefox'), findsOneWidget);
    expect(find.text('Files'), findsOneWidget);
  });

  testWidgets('a whitespace query still shows the tabs, never the app list', (
    tester,
  ) async {
    fake.frequentAppsResults = (limit) => [usage('Firefox')];
    await pumpSearch(tester);

    await tester.enterText(find.byKey(AppSearch.fieldKey), '   ');
    await tester.pump();
    await tester.pump();

    expect(find.byKey(AppSearch.recentTabKey), findsOneWidget);
    expect(fake.searchQueries, isEmpty);
    expect(fake.usageQueries.last, 'frequent(24)');
  });

  testWidgets('the Recent tab asks the core for recent apps', (tester) async {
    fake.recentAppsResults = (limit) => [usage('Zed')];
    await pumpSearch(tester);

    await tester.tap(find.byKey(AppSearch.recentTabKey));
    await tester.pump();
    await tester.pump();

    expect(fake.usageQueries.last, 'recent(24)');
    expect(find.text('Zed'), findsOneWidget);
  });

  testWidgets('typing hides the tabs and shows the search results', (
    tester,
  ) async {
    fake.searchAllAppsResults = (q) =>
        q == 'fir' ? [hit('Firefox')] : [hit('Files')];
    await pumpSearch(tester);

    await tester.enterText(find.byKey(AppSearch.fieldKey), 'fir');
    await tester.pump();
    await tester.pump();

    expect(fake.searchQueries.last, 'fir');
    expect(find.byKey(AppSearch.frequentTabKey), findsNothing);
    expect(find.text('Firefox'), findsOneWidget);
  });

  testWidgets('the gear asks to open settings and refocuses on return', (
    tester,
  ) async {
    var opened = false;
    await pumpSearch(tester, onOpenSettings: () async => opened = true);

    field(tester).focusNode!.unfocus();
    await tester.pump();
    expect(field(tester).focusNode!.hasFocus, isFalse);

    await tester.tap(find.byKey(AppSearch.gearKey));
    await tester.pump();
    await tester.pump();

    expect(opened, isTrue);
    expect(field(tester).focusNode!.hasFocus, isTrue);
  });

  testWidgets('tapping a result launches the app and closes', (tester) async {
    var closed = false;
    fake.searchAllAppsResults = (q) => [
      hit('Firefox', package: 'org.mozilla.firefox'),
    ];
    await pumpSearch(tester, onDone: () => closed = true);

    await tester.enterText(find.byKey(AppSearch.fieldKey), 'fir');
    await tester.pump();
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
    await pumpSearch(tester, onDone: () => closed = true);

    await tester.tap(find.byKey(AppSearch.closeKey));
    await tester.pump();

    expect(closed, isTrue);
    expect(fake.launched, isEmpty);
  });

  testWidgets('the search box and the cells take the Font size', (tester) async {
    fake.current = withSettings(
      snapshotOf(pageIds: ['apps'], groups: [makeGroup('apps')]),
      settingsWith(labelSize: 24),
    );
    fake.frequentAppsResults = (limit) => [usage('Firefox')];
    await pumpSearch(tester);
    await tester.pump();

    expect(field(tester).style?.fontSize, 24);
    expect(
      tester
          .renderObject<RenderParagraph>(find.text('Firefox'))
          .text
          .style
          ?.fontSize,
      24,
    );
  });

  testWidgets('no results is an empty grid, not an error', (tester) async {
    fake.searchAllAppsResults = (q) => const [];
    await pumpSearch(tester);

    await tester.enterText(find.byKey(AppSearch.fieldKey), 'zzz');
    await tester.pump();
    await tester.pump();

    expect(find.text('zzz'), findsOneWidget); // the field still holds it
    expect(tester.takeException(), isNull);
  });
}
