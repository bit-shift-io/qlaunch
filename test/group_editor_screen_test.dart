import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/group_editor_screen.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() {
    fake = FakeBackend(
      snapshot: snapshotOf(
        rootGroupId: 'root',
        pageIds: ['apps', 'games', 'tools'],
        groups: [
          Group(id: 'root', label: 'root', order: 0),
          makeGroup('apps', role: GroupRole.apps, order: 0),
          makeGroup('games', role: GroupRole.games, order: 1),
          makeGroup('tools', order: 2),
        ],
      ),
    );
    state = LauncherState(fake);
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pumpEditor(WidgetTester tester) async {
    await state.load();
    await tester.pumpWidget(MaterialApp(home: GroupEditorScreen(state: state)));
    await tester.pump();
  }

  testWidgets('lists each page in order', (tester) async {
    await pumpEditor(tester);

    expect(find.byKey(GroupEditorScreen.rowKey('apps')), findsOneWidget);
    expect(find.byKey(GroupEditorScreen.rowKey('games')), findsOneWidget);
    expect(find.byKey(GroupEditorScreen.rowKey('tools')), findsOneWidget);
  });

  testWidgets('the first page cannot move up and the last cannot move down', (
    tester,
  ) async {
    await pumpEditor(tester);

    expect(
      tester
          .widget<IconButton>(find.byKey(GroupEditorScreen.upKey('apps')))
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<IconButton>(find.byKey(GroupEditorScreen.downKey('tools')))
          .onPressed,
      isNull,
    );
  });

  testWidgets('moving the first page down sends the next index', (tester) async {
    await pumpEditor(tester);

    await tester.tap(find.byKey(GroupEditorScreen.downKey('apps')));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('moveGroup(apps,1)'));
  });

  testWidgets('adding a group sends the typed label', (tester) async {
    await pumpEditor(tester);

    await tester.tap(find.byKey(GroupEditorScreen.addKey));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(GroupEditorScreen.addFieldKey), 'Tools');
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(GroupEditorScreen.addConfirmKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('addGroup(Tools)'));
  });

  Future<void> openEdit(WidgetTester tester, String id) async {
    await tester.tap(find.byKey(GroupEditorScreen.rowKey(id)));
    await tester.pumpAndSettle();
  }

  testWidgets('renaming a tab sends the new label', (tester) async {
    await pumpEditor(tester);
    await openEdit(tester, 'tools');

    await tester.enterText(
      find.byKey(GroupEditorScreen.editLabelFieldKey),
      'Stuff',
    );
    await tester.tap(find.byKey(GroupEditorScreen.editSaveKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('renameGroup(tools,Stuff)'));
  });

  testWidgets('assigning the Home role sends setGroupRole', (tester) async {
    await pumpEditor(tester);
    await openEdit(tester, 'tools');

    await tester.tap(find.byKey(GroupEditorScreen.roleKey(GroupRole.home)));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('setGroupRole(tools,home)'));
  });

  testWidgets('the role control is disabled for the Apps group', (tester) async {
    await pumpEditor(tester);
    await openEdit(tester, 'apps');

    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(GroupEditorScreen.roleKey(GroupRole.home)),
          )
          .onSelected,
      isNull,
    );
  });

  testWidgets('deleting an ordinary group asks first', (tester) async {
    await pumpEditor(tester);
    await openEdit(tester, 'tools');

    await tester.tap(find.byKey(GroupEditorScreen.editDeleteKey));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(GroupEditorScreen.confirmDeleteKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('deleteGroup(tools)'));
  });
}
