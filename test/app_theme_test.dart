import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/main.dart';
import 'package:qlaunch/src/ui/launcher_menu.dart';
import 'package:qlaunch/src/ui/settings_screen.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  FakeBackend backend() => FakeBackend(
    snapshot: snapshotOf(pageIds: ['home'], groups: [makeGroup('home')]),
  );

  testWidgets('the app defaults to a dark theme', (tester) async {
    final fake = backend();
    addTearDown(fake.close);

    await tester.pumpWidget(QLaunchApp(backendOverride: fake));
    await tester.pump();

    final app = tester.widget<MaterialApp>(find.byType(MaterialApp));
    expect(app.theme?.brightness, Brightness.dark);
    expect(app.themeMode, ThemeMode.dark);
  });

  testWidgets('the settings screen is drawn on the dark theme', (tester) async {
    final fake = backend();
    addTearDown(fake.close);
    tester.view.physicalSize = const Size(400, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(QLaunchApp(backendOverride: fake));
    await tester.pump();
    await tester.pump();

    final grid = tester.getRect(find.byType(GridView));
    await tester.longPressAt(Offset(grid.left + 30, grid.bottom - 30));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(LauncherMenu.settingsKey));
    await tester.pumpAndSettle();

    expect(find.byType(SettingsScreen), findsOneWidget);
    final context = tester.element(find.byType(SettingsScreen));
    expect(Theme.of(context).brightness, Brightness.dark);
  });
}
