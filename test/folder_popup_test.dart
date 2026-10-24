import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_settings_dialog.dart';
import 'package:qlaunch/src/ui/folder_popup.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() {
    fake = FakeBackend(
      snapshot: snapshotOf(
        rootGroupId: 'root',
        groups: [
          makeGroup('root'),
          makeGroup('apps', role: GroupRole.apps),
          makeGroup('chat', folder: true, label: 'Chat'),
        ],
        items: [
          makeItem('chat-tile', 'apps', folderGroupId: 'chat'),
          makeItem(
            'm1',
            'chat',
            app: AppRef(package: 'pkg.m1', activity: 'Main'),
          ),
          makeItem(
            'm2',
            'chat',
            app: AppRef(package: 'pkg.m2', activity: 'Main'),
          ),
        ],
      ),
    );
    state = LauncherState(fake);
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pumpPopup(WidgetTester tester, {String folder = 'chat'}) async {
    await state.load();
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: TextButton(
              onPressed: () => FolderPopup.show(context, state, folder),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
  }

  testWidgets('shows its members and nothing else', (tester) async {
    await pumpPopup(tester);

    expect(find.text('m1'), findsOneWidget);
    expect(find.text('m2'), findsOneWidget);
    expect(find.text('Chat'), findsNothing);
    expect(find.byIcon(Icons.close), findsNothing);
  });

  testWidgets('sizes itself to fit the members', (tester) async {
    await pumpPopup(tester);
    final two = tester.getSize(find.byKey(FolderPopup.contentKey));

    fake.current = snapshotOf(
      rootGroupId: 'root',
      groups: [
        makeGroup('root'),
        makeGroup('apps', role: GroupRole.apps),
        makeGroup('chat', folder: true, label: 'Chat'),
      ],
      items: [
        makeItem('chat-tile', 'apps', folderGroupId: 'chat'),
        makeItem(
          'm1',
          'chat',
          app: AppRef(package: 'pkg.m1', activity: 'Main'),
        ),
      ],
    );
    await state.load();
    await tester.pump();
    final one = tester.getSize(find.byKey(FolderPopup.contentKey));

    expect(one.width, lessThan(two.width));
    expect(two.width, 2 * one.width);
    expect(one.height, two.height);
  });

  testWidgets('the members are not scrollable when they fit', (tester) async {
    tester.view.padding = const FakeViewPadding(top: 52, bottom: 24);
    addTearDown(tester.view.resetPadding);
    await pumpPopup(tester);

    final grid = tester.state<ScrollableState>(
      find.descendant(
        of: find.byKey(FolderPopup.contentKey),
        matching: find.byType(Scrollable),
      ),
    );
    expect(grid.position.maxScrollExtent, 0);
  });

  testWidgets('tapping a member launches it', (tester) async {
    await pumpPopup(tester);

    await tester.tap(find.text('m1'));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('launchApp(pkg.m1,Main)'));
  });

  testWidgets('long-pressing a member opens the app settings dialog', (
    tester,
  ) async {
    await pumpPopup(tester);

    await tester.longPress(find.text('m1'));
    await tester.pumpAndSettle();

    expect(find.byType(AppSettingsDialog), findsOneWidget);
  });

  testWidgets('an empty folder says so', (tester) async {
    fake.current = snapshotOf(
      rootGroupId: 'root',
      groups: [
        makeGroup('root'),
        makeGroup('apps', role: GroupRole.apps),
        makeGroup('chat', folder: true, label: 'Chat'),
      ],
      items: [makeItem('chat-tile', 'apps', folderGroupId: 'chat')],
    );
    await pumpPopup(tester);

    expect(find.byKey(FolderPopup.emptyKey), findsOneWidget);
    expect(find.text('Empty folder'), findsOneWidget);
  });
}
