import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/api/shortcuts.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/launcher_menu.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() async {
    fake = FakeBackend(
      snapshot: snapshotOf(pageIds: ['home'], groups: [makeGroup('home')]),
    );
    state = LauncherState(fake);
    await state.load();
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pumpDialog(WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: TextButton(
              onPressed: () => showDialog<void>(
                context: context,
                builder: (context) =>
                    AddShortcutDialog(state: state, targetGroupId: 'home'),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
  }

  testWidgets('the lock action targets the page it was opened from', (
    tester,
  ) async {
    await pumpDialog(tester);

    await tester.tap(find.byKey(AddShortcutDialog.lockKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('addLockShortcut(home)'));
  });

  testWidgets('the notifications action targets the page it was opened from', (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    try {
      await pumpDialog(tester);

      await tester.tap(find.byKey(AddShortcutDialog.notificationsKey));
      await tester.pumpAndSettle();

      expect(fake.calls, contains('addNotificationsShortcut(home)'));
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });

  testWidgets('the notifications action is hidden off Android', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    try {
      await pumpDialog(tester);

      expect(find.byKey(AddShortcutDialog.notificationsKey), findsNothing);
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });

  testWidgets('a command shortcut sends its name and command', (tester) async {
    fake.shortcutProviderResults = const [
      ShortcutProvider(id: 'command', label: 'Command'),
    ];
    await pumpDialog(tester);

    await tester.tap(find.byKey(AddShortcutDialog.commandKey));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(AddShortcutDialog.commandLabelKey),
      'Terminal',
    );
    await tester.enterText(
      find.byKey(AddShortcutDialog.commandTextKey),
      'xterm -e top',
    );
    await tester.pump();
    await tester.tap(find.byKey(AddShortcutDialog.commandSaveKey));
    await tester.pumpAndSettle();

    expect(
      fake.calls,
      contains('addCommandShortcut(home,Terminal,xterm -e top)'),
    );
  });

  testWidgets('an Android provider starts its create flow', (tester) async {
    fake.shortcutProviderResults = const [
      ShortcutProvider(id: 'pkg/.Provider', label: 'Books'),
    ];
    await pumpDialog(tester);

    await tester.tap(
      find.byKey(AddShortcutDialog.providerKey('pkg/.Provider')),
    );
    await tester.pumpAndSettle();

    expect(fake.calls, contains('beginShortcut(pkg/.Provider,home)'));
  });
}
