import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_settings_dialog.dart';

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
          makeGroup('apps', role: GroupRole.apps),
          makeGroup('games', role: GroupRole.games),
        ],
        items: [
          makeItem('i1', 'apps', app: AppRef(package: 'com.x', activity: 'Main')),
        ],
      ),
    );
    state = LauncherState(fake);
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pumpDialog(WidgetTester tester) async {
    await state.load();
    final item = state.itemsIn('apps').single;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AppSettingsDialog(state: state, item: item),
        ),
      ),
    );
    await tester.pump();
  }

  testWidgets('saving a new label renames the item', (tester) async {
    await pumpDialog(tester);

    await tester.enterText(find.byKey(AppSettingsDialog.labelFieldKey), 'Firefox');
    await tester.tap(find.byKey(AppSettingsDialog.saveKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('renameItem(i1,Firefox)'));
  });

  testWidgets('save is disabled when the label is blank', (tester) async {
    await pumpDialog(tester);

    await tester.enterText(find.byKey(AppSettingsDialog.labelFieldKey), '   ');
    await tester.pump();

    final save = tester.widget<FilledButton>(
      find.byKey(AppSettingsDialog.saveKey),
    );
    expect(save.onPressed, isNull);
  });

  testWidgets('choosing another group moves the item to the end', (
    tester,
  ) async {
    await pumpDialog(tester);

    await tester.tap(find.byKey(AppSettingsDialog.groupKey));
    await tester.pumpAndSettle();
    await tester.tap(find.text('games').hitTestable());
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(AppSettingsDialog.saveKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('moveItem(i1,games,0)'));
  });

  testWidgets('saving without changing anything sends no move or rename', (
    tester,
  ) async {
    await pumpDialog(tester);

    await tester.tap(find.byKey(AppSettingsDialog.saveKey));
    await tester.pumpAndSettle();

    expect(fake.calls.where((c) => c.startsWith('moveItem')), isEmpty);
    expect(fake.calls.where((c) => c.startsWith('renameItem')), isEmpty);
  });

  testWidgets('uninstall asks the backend for the package', (tester) async {
    await pumpDialog(tester);

    await tester.tap(find.byKey(AppSettingsDialog.uninstallKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('uninstallApp(com.x)'));
  });

  testWidgets('an item with no app hides the uninstall button', (tester) async {
    fake.current = snapshotOf(
      pageIds: ['apps'],
      groups: [makeGroup('apps', role: GroupRole.apps)],
      items: [makeItem('i1', 'apps')],
    );
    await state.load();
    final item = state.itemsIn('apps').single;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AppSettingsDialog(state: state, item: item),
        ),
      ),
    );
    await tester.pump();

    expect(find.byKey(AppSettingsDialog.uninstallKey), findsNothing);
  });
}
