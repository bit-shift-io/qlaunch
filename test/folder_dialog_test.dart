import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/folder_dialog.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() async {
    fake = FakeBackend(
      snapshot: snapshotOf(
        rootGroupId: 'root',
        pageIds: ['apps', 'games'],
        groups: [
          makeGroup('root'),
          makeGroup('apps', role: GroupRole.apps),
          makeGroup('games', role: GroupRole.games),
          makeGroup('hidden', role: GroupRole.hidden),
          makeGroup('chat', folder: true, label: 'Chat'),
        ],
        items: [
          makeItem('chat-tile', 'apps', folderGroupId: 'chat'),
          makeItem('m1', 'chat'),
        ],
      ),
    );
    state = LauncherState(fake);
    await state.load();
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pumpDialog(WidgetTester tester, Widget dialog) async {
    await state.load();
    await tester.pumpWidget(MaterialApp(home: Scaffold(body: dialog)));
    await tester.pump();
  }

  testWidgets('create mode calls addFolder with the name', (tester) async {
    await pumpDialog(
      tester,
      FolderDialog.create(state: state, containingGroupId: 'apps'),
    );

    await tester.enterText(find.byKey(FolderDialog.nameKey), 'Chat');
    await tester.pump();
    await tester.tap(find.byKey(FolderDialog.saveKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('addFolder(apps,Chat)'));
  });

  testWidgets('save is disabled when the name is blank', (tester) async {
    await pumpDialog(
      tester,
      FolderDialog.create(state: state, containingGroupId: 'apps'),
    );

    await tester.enterText(find.byKey(FolderDialog.nameKey), '   ');
    await tester.pump();
    await tester.pump();

    final save = tester.widget<FilledButton>(
      find.byKey(FolderDialog.saveKey),
    );
    expect(save.onPressed, isNull);
  });

  testWidgets('edit mode renames the folder and its tile', (tester) async {
    await pumpDialog(
      tester,
      FolderDialog.edit(state: state, folderGroup: state.groupById('chat')!),
    );

    await tester.enterText(find.byKey(FolderDialog.nameKey), 'Friends');
    await tester.pump();
    await tester.tap(find.byKey(FolderDialog.saveKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('renameGroup(chat,Friends)'));
    expect(fake.calls.where((c) => c.startsWith('moveItem')), isEmpty);
  });

  testWidgets('edit mode moves the tile to the chosen group', (tester) async {
    await pumpDialog(
      tester,
      FolderDialog.edit(state: state, folderGroup: state.groupById('chat')!),
    );

    await tester.tap(find.byKey(FolderDialog.groupKey));
    await tester.pumpAndSettle();
    await tester.tap(find.text('games').hitTestable());
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(FolderDialog.saveKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('moveItem(chat-tile,games,0)'));
  });

  testWidgets('delete mode removes the folder', (tester) async {
    await pumpDialog(
      tester,
      FolderDialog.edit(state: state, folderGroup: state.groupById('chat')!),
    );

    await tester.tap(find.byKey(FolderDialog.deleteKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('deleteFolder(chat)'));
  });

  testWidgets('destinations omit the root, folders and the Hidden group', (
    tester,
  ) async {
    await pumpDialog(
      tester,
      FolderDialog.create(state: state, containingGroupId: 'apps'),
    );

    await tester.tap(find.byKey(FolderDialog.groupKey));
    await tester.pumpAndSettle();

    expect(find.byKey(FolderDialog.groupOptionKey('apps')), findsWidgets);
    expect(find.byKey(FolderDialog.groupOptionKey('games')), findsWidgets);
    expect(find.byKey(FolderDialog.groupOptionKey('root')), findsNothing);
    expect(find.byKey(FolderDialog.groupOptionKey('chat')), findsNothing);
    expect(find.byKey(FolderDialog.groupOptionKey('hidden')), findsNothing);
  });

  testWidgets('the current group is offered even when normally excluded', (
    tester,
  ) async {
    fake.current = snapshotOf(
      rootGroupId: 'root',
      groups: [
        makeGroup('root'),
        makeGroup('apps', role: GroupRole.apps),
        makeGroup('hidden', role: GroupRole.hidden),
        makeGroup('chat', folder: true),
      ],
      items: [
        makeItem('chat-tile', 'hidden', folderGroupId: 'chat'),
      ],
    );
    await pumpDialog(
      tester,
      FolderDialog.edit(state: state, folderGroup: state.groupById('chat')!),
    );

    await tester.tap(find.byKey(FolderDialog.groupKey));
    await tester.pumpAndSettle();

    expect(find.byKey(FolderDialog.groupOptionKey('hidden')), findsWidgets);
  });
}
