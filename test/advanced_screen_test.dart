import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/state/all_files_access.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/advanced_screen.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  late FakeBackend fake;
  late LauncherState state;
  late bool storageGranted;
  late bool requestGrants;

  void mockStorage() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(AllFilesAccess.channel, (call) async {
          switch (call.method) {
            case 'hasAllFilesAccess':
              return storageGranted;
            case 'requestAllFilesAccess':
              storageGranted = requestGrants;
              return null;
          }
          return null;
        });
  }

  setUp(() async {
    storageGranted = true;
    requestGrants = true;
    mockStorage();
    fake = FakeBackend(
      snapshot: snapshotOf(pageIds: ['home'], groups: [makeGroup('home')]),
    );
    state = LauncherState(fake);
    await state.load();
  });

  tearDown(() async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(AllFilesAccess.channel, null);
    state.dispose();
    await fake.close();
  });

  Future<void> pumpAdvanced(WidgetTester tester) async {
    await tester.pumpWidget(MaterialApp(home: AdvancedScreen(state: state)));
    await tester.pump();
  }

  Future<void> tapControl(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pump();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  testWidgets('every action has a row', (tester) async {
    await pumpAdvanced(tester);

    expect(find.byKey(AdvancedScreen.restartKey), findsOneWidget);
    expect(find.byKey(AdvancedScreen.backupKey), findsOneWidget);
    expect(find.byKey(AdvancedScreen.restoreKey), findsOneWidget);
    expect(find.byKey(AdvancedScreen.factoryResetKey), findsOneWidget);
  });

  testWidgets('Restart Launcher reads the model again', (tester) async {
    await pumpAdvanced(tester);
    final before = fake.calls.where((c) => c == 'init').length;

    await tapControl(tester, find.byKey(AdvancedScreen.restartKey));

    expect(fake.calls.where((c) => c == 'init').length, before + 1);
  });

  testWidgets('Backup config names the file it wrote', (tester) async {
    fake.backupName = 'qlaunch-backup-2026-10-21-140530.json';
    await pumpAdvanced(tester);

    await tapControl(tester, find.byKey(AdvancedScreen.backupKey));

    expect(fake.calls, contains('backupConfig'));
    expect(
      find.text(
        'Backup saved to Downloads/qlaunch-backup-2026-10-21-140530.json.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('a failed backup says so', (tester) async {
    await pumpAdvanced(tester);
    final reported = captureReports();
    fake.failure = StateError('the disk is full');

    await tapControl(tester, find.byKey(AdvancedScreen.backupKey));

    expect(reported, hasLength(1));
    expect(find.textContaining('Backup failed'), findsOneWidget);
  });

  testWidgets('Restore with no backups says so and calls nothing', (
    tester,
  ) async {
    fake.backups = const [];
    await pumpAdvanced(tester);

    await tapControl(tester, find.byKey(AdvancedScreen.restoreKey));

    expect(find.text('No backups found in Downloads.'), findsOneWidget);
    expect(fake.calls.where((c) => c.startsWith('restoreConfig')), isEmpty);
  });

  testWidgets('Restore asks for All files access when denied', (tester) async {
    storageGranted = false;
    requestGrants = false;
    fake.backups = ['qlaunch-backup-2026-10-21-140530.json'];
    await pumpAdvanced(tester);

    await tapControl(tester, find.byKey(AdvancedScreen.restoreKey));

    expect(find.textContaining('All files access'), findsOneWidget);
    expect(fake.calls.where((c) => c == 'listBackups'), isEmpty);
    expect(fake.calls.where((c) => c.startsWith('restoreConfig')), isEmpty);
  });

  testWidgets('Restore lists backups once access is granted', (tester) async {
    storageGranted = false;
    fake.backups = ['qlaunch-backup-2026-10-21-140530.json'];
    await pumpAdvanced(tester);

    // The request handler flips granted=true, as granting in Settings would.
    await tapControl(tester, find.byKey(AdvancedScreen.restoreKey));

    expect(find.text('qlaunch-backup-2026-10-21-140530.json'), findsOneWidget);
    expect(fake.calls, contains('listBackups'));
  });

  testWidgets('Restore lists backups and restores the chosen one', (
    tester,
  ) async {
    fake.backups = [
      'qlaunch-backup-2026-10-21-140530.json',
      'qlaunch-backup-2026-10-20-090000.json',
    ];
    await pumpAdvanced(tester);

    await tapControl(tester, find.byKey(AdvancedScreen.restoreKey));
    expect(find.text('qlaunch-backup-2026-10-21-140530.json'), findsOneWidget);
    expect(find.text('qlaunch-backup-2026-10-20-090000.json'), findsOneWidget);

    await tester.tap(find.text('qlaunch-backup-2026-10-20-090000.json'));
    await tester.pumpAndSettle();
    expect(
      find.textContaining('This replaces your current layout'),
      findsOneWidget,
    );

    await tester.tap(find.text('Restore'));
    await tester.pumpAndSettle();

    expect(
      fake.calls,
      contains('restoreConfig(qlaunch-backup-2026-10-20-090000.json)'),
    );
    expect(
      find.text('Config restored from qlaunch-backup-2026-10-20-090000.json.'),
      findsOneWidget,
    );
  });

  testWidgets('cancelling the restore confirmation changes nothing', (
    tester,
  ) async {
    fake.backups = ['qlaunch-backup-2026-10-21-140530.json'];
    await pumpAdvanced(tester);

    await tapControl(tester, find.byKey(AdvancedScreen.restoreKey));
    await tester.tap(find.text('qlaunch-backup-2026-10-21-140530.json'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    expect(fake.calls.where((c) => c.startsWith('restoreConfig')), isEmpty);
  });

  group('factory reset', () {
    testWidgets('asks first, and calls nothing', (tester) async {
      await pumpAdvanced(tester);
      final calls = List.of(fake.calls);

      await tester.tap(find.byKey(AdvancedScreen.factoryResetKey));
      await tester.pumpAndSettle();

      expect(find.textContaining('cannot be undone'), findsOneWidget);
      expect(fake.calls, calls);

      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(find.textContaining('cannot be undone'), findsNothing);
      expect(fake.calls, calls);
    });

    testWidgets('confirming resets the launcher', (tester) async {
      await pumpAdvanced(tester);

      await tester.tap(find.byKey(AdvancedScreen.factoryResetKey));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Reset'));
      await tester.pumpAndSettle();

      expect(fake.calls, contains('factoryReset'));
      expect(find.textContaining('was reset'), findsOneWidget);
    });
  });
}
