// End-to-end tests for the Linux side (§8.8): the whole stack, from the
// applications on a machine to an icon on a screen and a launched process.
//
//   flutter test integration_test/linux_launcher_test.dart
//
// Two halves. The first runs the real UI over a fake core, so it is
// deterministic and passes anywhere. The second runs it over the real core,
// with the `.desktop` files in `test/fixtures/applications` standing in for the
// machine's applications, so it needs nothing from the machine but a built
// `rust/target/release/libqlaunch_core.so`.
//
// The second half points the core at those fixtures through the XDG variables
// the core reads, and moves the configuration root to a temporary directory, so
// it never reads or writes the user's own document. It sets the variables
// rather than passing paths in because `init` is required to resolve them
// itself; Dart does not compute them, not even here.
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:qlaunch/main.dart';
import 'package:qlaunch/src/rust/api/api.dart' as rust;
import 'package:qlaunch/src/rust/frb_generated.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/ui/app_cell.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

import '../test/fake_backend.dart';
import '../test/snapshot_builders.dart';

/// Sets a process environment variable, for the core to read.
///
/// `Platform.environment` is read-only and there is no Dart API for this, so it
/// reaches libc directly. The two small allocations are deliberately leaked:
/// the test process is short-lived and it keeps this to one lookup each.
void _setEnv(String name, String value) {
  final libc = DynamicLibrary.process();
  final malloc = libc
      .lookupFunction<
        Pointer<Uint8> Function(IntPtr),
        Pointer<Uint8> Function(int)
      >('malloc');
  final setenv = libc
      .lookupFunction<
        Int32 Function(Pointer<Uint8>, Pointer<Uint8>, Int32),
        int Function(Pointer<Uint8>, Pointer<Uint8>, int)
      >('setenv');

  Pointer<Uint8> cString(String text) {
    final bytes = utf8.encode(text);
    final pointer = malloc(bytes.length + 1);
    final view = pointer.asTypedList(bytes.length + 1);
    view.setAll(0, bytes);
    view[bytes.length] = 0;
    return pointer;
  }

  if (setenv(cString(name), cString(value), 1) != 0) {
    throw StateError('could not set $name');
  }
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  AppRef app(String package) => AppRef(package: package, activity: '');

  /// What [FakeBackend.launched] records for an app on Linux: the desktop file
  /// id, and no activity.
  ({String package, String activity}) launched(String package) =>
      (package: package, activity: '');

  group('over a fake core', () {
    late FakeBackend fake;

    setUp(() {
      fake = FakeBackend(
        snapshot: snapshotOf(
          homeGroupId: 'home',
          pageIds: const ['apps'],
          groups: [
            makeGroup('root', role: GroupRole.root),
            makeGroup('home', role: GroupRole.home),
            makeGroup('apps', role: GroupRole.apps),
          ],
          items: [
            makeItem('Firefox', 'apps', app: app('firefox.desktop')),
            makeItem('No Icon Here', 'apps', app: app('noicon.desktop')),
          ],
        ),
      );
    });

    tearDown(() => fake.close());

    testWidgets('the launcher draws the apps and a tap launches one', (
      tester,
    ) async {
      await tester.pumpWidget(QLaunchApp(backendOverride: fake));
      await tester.pumpAndSettle();

      expect(find.text('Firefox'), findsOneWidget);
      expect(find.text('No Icon Here'), findsOneWidget);

      await tester.tap(find.text('Firefox'));
      await tester.pump();

      expect(fake.launched, [launched('firefox.desktop')]);
    });

    testWidgets('an app with no icon gets a placeholder, not a failure', (
      tester,
    ) async {
      await tester.pumpWidget(QLaunchApp(backendOverride: fake));
      await tester.pumpAndSettle();

      expect(find.byType(AppCell), findsNWidgets(2));
      expect(find.byKey(AppCell.placeholderKey), findsNWidgets(2));
      expect(tester.takeException(), isNull);
    });
  });

  group('over the real core and the fixture applications', () {
    late Directory configRoot;

    setUpAll(() async {
      configRoot = Directory.systemTemp.createTempSync('qlaunch-linux-e2e-');
      // The highest-priority data root is the fixture directory, so discovery
      // sees exactly the five fixture files and nothing of the machine's.
      _setEnv('XDG_DATA_DIRS', Directory('test/fixtures').absolute.path);
      _setEnv('XDG_DATA_HOME', '${configRoot.path}/data-home');
      _setEnv('XDG_CONFIG_HOME', configRoot.path);

      await RustLib.init(
        externalLibrary: await loadExternalLibrary(
          const ExternalLibraryLoaderConfig(
            stem: 'qlaunch_core',
            ioDirectory: 'rust/target/release/',
            webPrefix: 'pkg/',
            wasmBindgenName: 'wasm_bindgen',
          ),
        ),
      );
    });

    tearDownAll(() {
      if (configRoot.existsSync()) configRoot.deleteSync(recursive: true);
    });

    testWidgets('the core discovers, filters and categorises the fixtures', (
      tester,
    ) async {
      final snapshot = await rust.init();

      final labels = snapshot.items.map((item) => item.label).toList();
      expect(labels, containsAll(['Firefox', 'Super Game', 'No Icon Here']));
      expect(labels, isNot(contains('Hidden One')));
      expect(labels, isNot(contains('No Display One')));

      // A game category puts the app on the Games page, the rest on Apps.
      final games = snapshot.groups.firstWhere(
        (group) => group.role == GroupRole.games,
      );
      final inGames = snapshot.items
          .where((item) => item.groupId == games.id)
          .map((item) => item.label);
      expect(inGames, ['Super Game']);

      // The first page is the Apps page, and it is the one on screen.
      expect(snapshot.pageIds, hasLength(3));
      final firefox = snapshot.items.firstWhere(
        (item) => item.label == 'Firefox',
      );
      expect(firefox.app!.package, 'firefox.desktop');
      expect(firefox.app!.activity, isEmpty);

      // No icon theme is reachable from the fixtures, so this is a miss rather
      // than a failure.
      expect(
        await rust.iconFor(
          package: firefox.app!.package,
          activity: firefox.app!.activity,
          sizePx: 48,
        ),
        isNull,
      );

      // The real launch path runs the real `Exec` (`/bin/true`).
      await rust.launchApp(
        package: firefox.app!.package,
        activity: firefox.app!.activity,
      );
    });

    testWidgets('the launcher shows them and launches one without complaint', (
      tester,
    ) async {
      await tester.pumpWidget(const QLaunchApp());
      await tester.pumpAndSettle();

      expect(find.byType(TabStrip), findsOneWidget);
      expect(find.text('Firefox'), findsOneWidget);
      expect(find.text('Super Game'), findsNothing); // that is the Games page

      await tester.tap(find.text('Firefox'));
      await tester.pumpAndSettle();

      // A failed launch is the only thing that would put a message on screen.
      expect(find.byType(SnackBar), findsNothing);
    });
  });
}
