import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/icon.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cell.dart';
import 'package:qlaunch/src/ui/app_grid.dart';
import 'package:qlaunch/src/ui/scroll_edge_fade.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

final Uint8List _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
);

Item _appItem(
  String id, {
  String group = 'g',
  String? package,
  String activity = 'Main',
  int order = 0,
  bool uninstalled = false,
  bool shortcut = false,
  bool empty = false,
}) => Item(
  id: id,
  groupId: group,
  label: id,
  labelOverridden: false,
  app: shortcut || empty
      ? null
      : AppRef(package: package ?? 'pkg.$id', activity: activity),
  shortcut: shortcut ? const ShortcutPayload.lockScreen() : null,
  uninstalled: uninstalled,
  order: order,
);

void main() {
  late FakeBackend fake;
  late LauncherState state;
  late List<FlutterErrorDetails> reported;

  setUp(() {
    fake = FakeBackend();
    state = LauncherState(fake);
    reported = [];
    final previousOnError = FlutterError.onError;
    FlutterError.onError = reported.add;
    addTearDown(() => FlutterError.onError = previousOnError);
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> load(List<Item> items, {Settings? settings}) async {
    var snapshot = snapshotOf(
      pageIds: ['g'],
      groups: [makeGroup('g')],
      items: items,
    );
    if (settings != null) snapshot = withSettings(snapshot, settings);
    fake.current = snapshot;
    await state.load();
  }

  void setSurface(WidgetTester tester, Size logical, {double dpr = 1}) {
    tester.view.devicePixelRatio = dpr;
    tester.view.physicalSize = logical * dpr;
    addTearDown(tester.view.reset);
  }

  Future<void> pumpGrid(
    WidgetTester tester, {
    String groupId = 'g',
    ValueChanged<Item>? onItemTap,
    ValueChanged<Item>? onItemLongPress,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AppGrid(
            state: state,
            groupId: groupId,
            onItemTap: onItemTap,
            onItemLongPress: onItemLongPress,
          ),
        ),
      ),
    );
    await tester.pump();
  }

  int columns(WidgetTester tester) {
    final cells = find.byType(AppCell);
    final firstTop = tester.getTopLeft(cells.first).dy;
    return [
      for (var i = 0; i < cells.evaluate().length; i++)
        if (tester.getTopLeft(cells.at(i)).dy == firstTop) i,
    ].length;
  }

  List<Item> twelve() => [
    for (var i = 0; i < 12; i++) _appItem('a$i', order: i),
  ];

  group('columns', () {
    testWidgets('portrait follows grid_columns_portrait', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load(
        twelve(),
        settings: settingsWith(gridColumnsPortrait: 3, gridColumnsLandscape: 5),
      );
      await pumpGrid(tester);
      expect(columns(tester), 3);
    });

    testWidgets('landscape follows grid_columns_landscape', (tester) async {
      setSurface(tester, const Size(800, 400));
      await load(
        twelve(),
        settings: settingsWith(gridColumnsPortrait: 3, gridColumnsLandscape: 5),
      );
      await pumpGrid(tester);
      expect(columns(tester), 5);
    });

    testWidgets('a settings change re-lays the grid out', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load(twelve(), settings: settingsWith(gridColumnsPortrait: 3));
      await pumpGrid(tester);
      expect(columns(tester), 3);

      fake.current = withSettings(
        fake.current,
        settingsWith(gridColumnsPortrait: 2),
      );
      fake.emit(const CoreEvent.modelChanged());
      await tester.pump();
      await tester.pump();
      expect(columns(tester), 2);
    });

    testWidgets('cells use content-sized rows', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load(twelve(), settings: settingsWith(gridColumnsPortrait: 4));
      await pumpGrid(tester);
      expect(tester.getSize(find.byType(AppCell).first), const Size(100, 82));
    });
  });

  group('scroll fade', () {
    for (final mode in ScrollFadeStyle.values) {
      testWidgets('$mode wraps the grid the way it says', (tester) async {
        setSurface(tester, const Size(400, 800));
        await load(twelve(), settings: settingsWith(scrollFade: mode));
        await pumpGrid(tester);

        expect(
          find.byType(ScrollEdgeFade),
          mode == ScrollFadeStyle.gradient ? findsOneWidget : findsNothing,
        );
        expect(
          find.byType(TileEdgeFade),
          mode == ScrollFadeStyle.tile ? findsNWidgets(12) : findsNothing,
        );
      });
    }
  });

  group('items', () {
    testWidgets('they appear in the order the model gave, unsorted', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      // Deliberately not in `order` order: Rust has already sorted.
      await load([
        _appItem('zulu', order: 2),
        _appItem('alpha', order: 0),
        _appItem('mike', order: 1),
      ]);
      await pumpGrid(tester);
      final lefts = {
        for (final id in ['zulu', 'alpha', 'mike'])
          id: tester.getTopLeft(find.text(id)).dx,
      };
      expect(lefts['zulu']! < lefts['alpha']!, isTrue);
      expect(lefts['alpha']! < lefts['mike']!, isTrue);
    });

    testWidgets('only the group asked for is shown', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('mine'), _appItem('theirs', group: 'other')]);
      await pumpGrid(tester);
      expect(find.text('mine'), findsOneWidget);
      expect(find.text('theirs'), findsNothing);
    });

    testWidgets('a group with zero items renders an empty grid', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(const []);
      await pumpGrid(tester);
      expect(find.byType(GridView), findsOneWidget);
      expect(find.byType(AppCell), findsNothing);
      expect(tester.takeException(), isNull);
    });

    testWidgets('the empty-pages sentinel renders an empty grid', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('a')]);
      await pumpGrid(tester, groupId: '');
      expect(find.byType(AppCell), findsNothing);
      expect(tester.takeException(), isNull);
    });
  });

  group('tapping', () {
    testWidgets('a tap launches that item\'s package and activity', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([
        _appItem('first', package: 'org.one', activity: 'A'),
        _appItem('second', package: 'org.two', activity: 'B'),
      ]);
      await pumpGrid(tester);
      await tester.tap(find.text('second'));
      await tester.pump();
      expect(fake.launched, [(package: 'org.two', activity: 'B')]);
    });

    testWidgets('a shortcut launches through the core', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('sc', shortcut: true)]);
      await pumpGrid(tester);
      await tester.tap(find.text('sc'));
      await tester.pump();
      expect(fake.launched, isEmpty);
      expect(fake.calls, contains('launchShortcut(sc)'));
      expect(tester.takeException(), isNull);
    });

    testWidgets('an item with no app does nothing and does not throw', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('empty', empty: true)]);
      await pumpGrid(tester);
      await tester.tap(find.text('empty'));
      await tester.pump();
      expect(fake.launched, isEmpty);
      expect(fake.calls.where((c) => c.startsWith('launch')), isEmpty);
      expect(tester.takeException(), isNull);
    });

    testWidgets('an uninstalled item is shown but does not launch', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('gone', uninstalled: true)]);
      await pumpGrid(tester);
      expect(find.text('gone'), findsOneWidget);
      await tester.tap(find.text('gone'));
      await tester.pump();
      expect(fake.calls.where((c) => c.startsWith('launchApp')), isEmpty);
    });

    testWidgets('a failed launch is reported as state, not thrown', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('a')]);
      await pumpGrid(tester);
      final reported = captureReports();
      fake.failure = StateError('no such activity');
      await tester.tap(find.text('a'));
      await tester.pump();
      expect(state.error, contains('no such activity'));
      expect(reported, hasLength(1));
      expect(reported.single.exception, isA<StateError>());
    });

    testWidgets('onItemTap replaces the default launch', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('a')]);
      final tapped = <String>[];
      await pumpGrid(tester, onItemTap: (i) => tapped.add(i.id));
      await tester.tap(find.text('a'));
      expect(tapped, ['a']);
      expect(fake.launched, isEmpty);
    });

    testWidgets('long-press goes to onItemLongPress for every item kind', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([
        _appItem('app'),
        _appItem('sc', shortcut: true),
        _appItem('gone', uninstalled: true),
      ]);
      final pressed = <String>[];
      await pumpGrid(tester, onItemLongPress: (i) => pressed.add(i.id));
      for (final id in ['app', 'sc', 'gone']) {
        await tester.longPress(find.text(id));
      }
      expect(pressed, ['app', 'sc', 'gone']);
    });

    testWidgets(
      'a long-press with no handler does nothing and does not throw',
      (tester) async {
        setSurface(tester, const Size(400, 800));
        await load([_appItem('a')]);
        await pumpGrid(tester);
        await tester.longPress(find.text('a'));
        expect(tester.takeException(), isNull);
        expect(fake.launched, isEmpty);
      },
    );
  });

  group('icons', () {
    testWidgets('garbage png bytes show the placeholder, not an exception', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      fake.icons[(package: 'pkg.a', activity: 'Main')] = IconImage(
        format: IconFormat.png,
        width: 0,
        height: 0,
        bytes: Uint8List.fromList([1, 2, 3]),
      );
      await load([_appItem('a')]);
      await pumpGrid(tester);
      await tester.pumpAndSettle();
      expect(find.byKey(AppCell.placeholderKey), findsOneWidget);
      expect(tester.takeException(), isNull);
    });

    testWidgets('a page disposed and rebuilt does not fetch its icons again', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('a')]);
      await pumpGrid(tester);
      expect(fake.iconCalls, 1);
      await tester.pumpWidget(const SizedBox.shrink()); // page disposed
      await pumpGrid(tester); // rebuilt
      expect(fake.iconCalls, 1); // cache survived the widget
    });

    testWidgets('the backend icon bytes are what the cell draws', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      fake.icons[(package: 'pkg.a', activity: 'Main')] = IconImage(
        format: IconFormat.png,
        width: 0,
        height: 0,
        bytes: _png,
      );
      await load([_appItem('a')]);
      await pumpGrid(tester);
      expect(find.byType(Image), findsOneWidget);
    });

    testWidgets('icons from the backend are drawn', (tester) async {
      setSurface(tester, const Size(400, 800));
      fake.icons[(package: 'pkg.a', activity: 'Main')] = IconImage(
        format: IconFormat.png,
        width: 0,
        height: 0,
        bytes: _png,
      );
      await load([_appItem('a'), _appItem('b')]);
      await pumpGrid(tester);
      expect(find.byType(Image), findsOneWidget);
      expect(find.byKey(AppCell.placeholderKey), findsOneWidget);
    });

    testWidgets('the size asked for is icon_size in physical pixels', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800), dpr: 2.5);
      await load([_appItem('a')], settings: settingsWith(iconSize: 40));
      await pumpGrid(tester);
      expect(fake.calls.where((c) => c.startsWith('iconFor')), [
        'iconFor(pkg.a,Main,100)',
      ]);
    });

    testWidgets('two cells for the same app make one iconFor call', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([
        _appItem('first', package: 'org.same', activity: 'M'),
        _appItem('second', package: 'org.same', activity: 'M'),
        _appItem('third', package: 'org.other', activity: 'M'),
      ]);
      await pumpGrid(tester);
      expect(fake.iconCalls, 2);
    });

    testWidgets('rebuilding does not request an icon again', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('a')]);
      await pumpGrid(tester);
      expect(fake.iconCalls, 1);

      fake.emit(const CoreEvent.modelChanged());
      await tester.pump();
      await tester.pump();
      await pumpGrid(tester);
      expect(fake.calls.where((c) => c == 'snapshot'), isNotEmpty);
      expect(fake.iconCalls, 1);
    });

    testWidgets('a different pixel ratio is a different cache key', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('a')], settings: settingsWith(iconSize: 40));
      await pumpGrid(tester);
      expect(fake.iconCalls, 1);

      tester.view.devicePixelRatio = 2;
      await tester.pump();
      await tester.pump();
      expect(fake.calls.where((c) => c.startsWith('iconFor')), [
        'iconFor(pkg.a,Main,40)',
        'iconFor(pkg.a,Main,80)',
      ]);
    });

    testWidgets('no icon is requested for an item that cannot show one', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load([
        _appItem('sc', shortcut: true),
        _appItem('gone', uninstalled: true),
      ]);
      await pumpGrid(tester);
      expect(fake.iconCalls, 0);
    });

    testWidgets('no icon is requested while icons are hidden', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load([_appItem('a')], settings: settingsWith(iconsVisible: false));
      await pumpGrid(tester);
      expect(fake.iconCalls, 0);
      expect(find.text('a'), findsOneWidget);
    });
  });

  group('LauncherState.iconFor and launchApp', () {
    test('iconFor returns a provider over the bytes the backend has', () async {
      fake.icons[(package: 'p', activity: 'a')] = IconImage(
        format: IconFormat.png,
        width: 0,
        height: 0,
        bytes: _png,
      );
      final provider = await state.iconFor('p', 'a', 48);
      expect(provider, isA<ResizeImage>());
      expect((provider! as ResizeImage).imageProvider, isA<MemoryImage>());
      final memory = (provider as ResizeImage).imageProvider as MemoryImage;
      expect(memory.bytes, _png);
      expect(state.error, isNull);
    });

    test('a missing icon is null, and not an error', () async {
      expect(await state.iconFor('p', 'a', 48), isNull);
      expect(state.error, isNull);
      expect(reported, isEmpty);
    });

    test('iconFor never throws, never sets error, but reports a bug', () async {
      fake.failure = StateError('bridge exploded');
      expect(await state.iconFor('p', 'a', 48), isNull);
      expect(state.error, isNull);
      expect(reported, hasLength(1));
      expect(reported.single.exception, isA<StateError>());
    });

    test('launchApp reaches the backend and leaves error alone', () async {
      await state.launchApp('p', 'a');
      expect(fake.launched, [(package: 'p', activity: 'a')]);
      expect(state.error, isNull);
    });

    test('a launchApp failure is state and a report, never a throw', () async {
      var notified = 0;
      state.addListener(() => notified++);
      fake.failure = StateError('no such app');
      await state.launchApp('p', 'a');
      expect(state.error, contains('no such app'));
      expect(notified, 1);
      expect(reported, hasLength(1));
      expect(reported.single.exception, isA<StateError>());
    });
  });
}
