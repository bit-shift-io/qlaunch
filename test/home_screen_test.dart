import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/state/icon_warmer.dart';
import 'package:qlaunch/src/ui/app_cell.dart';
import 'package:qlaunch/src/ui/app_grid.dart';
import 'package:qlaunch/src/ui/app_search.dart';
import 'package:qlaunch/src/ui/app_settings_dialog.dart';
import 'package:qlaunch/src/ui/background_layer.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/home_screen.dart';
import 'package:qlaunch/src/ui/launcher_menu.dart';
import 'package:qlaunch/src/ui/settings_screen.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

import 'fake_backend.dart';
import 'raster_test_images.dart' as fixtures;
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

  void setSurface(WidgetTester tester, Size logical, {double dpr = 1}) {
    tester.view.devicePixelRatio = dpr;
    tester.view.physicalSize = logical * dpr;
    addTearDown(tester.view.reset);
  }

  /// An item with an app, so a tap on it launches something.
  Item launchable(String id, String groupId) => makeItem(
    id,
    groupId,
    app: AppRef(package: 'pkg.$id', activity: 'Main'),
  );

  Future<void> load({
    List<String> pageIds = const ['home', 'apps'],
    String? homeGroupId,
    List<Item>? items,
    Settings? settings,
  }) async {
    var snapshot = snapshotOf(
      pageIds: pageIds,
      homeGroupId: homeGroupId,
      groups: [for (final id in pageIds) makeGroup(id)],
      items: items ?? [for (final id in pageIds) launchable('item-$id', id)],
    );
    if (settings != null) snapshot = withSettings(snapshot, settings);
    fake.current = snapshot;
    await state.load();
  }

  Future<void> pumpHome(
    WidgetTester tester, {
    IconWarmer? warmer,
    RasterImageStore? rasterStore,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        home: HomeScreen(
          state: state,
          rasterStore: rasterStore,
          iconWarmer:
              warmer ?? IconWarmer(schedule: (task) => scheduleMicrotask(task)),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();
  }

  /// Lets a shown SnackBar expire and finish animating out. One round is not
  /// enough: the dismiss timer only starts once the entrance animation has
  /// completed, and the first pump is what completes it.
  Future<void> expireMessage(WidgetTester tester) async {
    for (var round = 0; round < 2; round++) {
      await tester.pump(const Duration(seconds: 5));
      await tester.pumpAndSettle();
    }
  }

  /// The pager's own scroll position. Deliberately white-box: re-anchoring is
  /// invisible in the group shown, so the position is the only honest measure
  /// of whether the pager was moved.
  double? pagerPage(WidgetTester tester) =>
      tester.widget<PageView>(find.byType(PageView)).controller?.page;

  group('continuous scrolling prototype', () {
    Future<void> openScroller(WidgetTester tester) async {
      setSurface(tester, const Size(400, 800));
      const ids = ['home', 'apps', 'games'];
      await load(
        pageIds: ids,
        settings: settingsWith(continuousScroll: true),
        items: [
          for (final id in ids)
            for (var i = 0; i < 24; i++) launchable('$id-$i', id),
        ],
      );
      await pumpHome(tester);
      await tester.pumpAndSettle();
    }

    ScrollController scrollController(WidgetTester tester) => tester
        .widget<CustomScrollView>(find.byKey(const ValueKey('groupScroller')))
        .controller!;

    String activeGroup(WidgetTester tester) =>
        tester.widget<TabStrip>(find.byType(TabStrip)).currentGroupId;

    testWidgets('scrolling wraps both ways and keeps the tabs in sync', (
      tester,
    ) async {
      await openScroller(tester);
      final scroll = scrollController(tester);
      expect(activeGroup(tester), 'home');
      final scroller = find.byKey(const ValueKey('groupScroller'));
      final heading = find
          .descendant(of: scroller, matching: find.text('home'))
          .first;
      expect(
        tester.getCenter(heading).dx,
        closeTo(tester.getCenter(scroller).dx, 0.1),
      );
      scroll.jumpTo(-100);
      await tester.pumpAndSettle();
      expect(activeGroup(tester), 'games');
      expect(scroll.offset, -100);
      scroll.jumpTo(2000);
      await tester.pumpAndSettle();
      expect(scroll.offset, 2000);
      expect(tester.takeException(), isNull);
    });

    testWidgets('tab taps and horizontal swipes scroll to section headings', (
      tester,
    ) async {
      await openScroller(tester);
      await tester.tap(find.byKey(TabStrip.tabKey(1)));
      await tester.pumpAndSettle();
      expect(activeGroup(tester), 'apps');
      final appsOffset = scrollController(tester).offset;
      expect(appsOffset, greaterThan(0));
      await tester.drag(
        find.byKey(const ValueKey('groupScroller')),
        const Offset(-200, 0),
      );
      await tester.pumpAndSettle();
      expect(activeGroup(tester), 'games');
      await tester.drag(
        find.byKey(const ValueKey('groupScroller')),
        const Offset(200, 0),
      );
      await tester.pumpAndSettle();
      expect(activeGroup(tester), 'apps');
      expect(scrollController(tester).offset, closeTo(appsOffset, 0.1));
      scrollController(tester).jumpTo(appsOffset + 100);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(TabStrip.tabKey(0)));
      await tester.pumpAndSettle();
      expect(scrollController(tester).offset, closeTo(appsOffset, 0.1));
    });

    testWidgets('changing the saved mode swaps navigation in place', (
      tester,
    ) async {
      await openScroller(tester);
      fake.current = withSettings(
        fake.current,
        settingsWith(continuousScroll: false),
      );
      await state.updateSettings(const SettingsPatch(continuousScroll: false));
      await tester.pumpAndSettle();
      expect(find.byType(PageView), findsOneWidget);
      expect(find.byKey(const ValueKey('groupScroller')), findsNothing);
      fake.current = withSettings(
        fake.current,
        settingsWith(continuousScroll: true),
      );
      await state.updateSettings(const SettingsPatch(continuousScroll: true));
      await tester.pumpAndSettle();
      expect(find.byType(PageView), findsNothing);
      expect(find.byKey(const ValueKey('groupScroller')), findsOneWidget);
    });
  });

  group('tab visibility', () {
    const shown = {
      TabVisibility.always: (portrait: true, landscape: true),
      TabVisibility.never: (portrait: false, landscape: false),
      TabVisibility.landscapeOnly: (portrait: false, landscape: true),
      TabVisibility.portraitOnly: (portrait: true, landscape: false),
    };

    for (final entry in shown.entries) {
      for (final orientation in const ['portrait', 'landscape']) {
        final expected = orientation == 'portrait'
            ? entry.value.portrait
            : entry.value.landscape;
        testWidgets('${entry.key.name} in $orientation', (tester) async {
          setSurface(
            tester,
            orientation == 'portrait'
                ? const Size(400, 800)
                : const Size(800, 400),
          );
          await load(settings: settingsWith(tabVisibility: entry.key));
          await pumpHome(tester);
          expect(
            find.byType(TabStrip),
            expected ? findsOneWidget : findsNothing,
          );
          // The grid is there either way: hiding the strip is not hiding the
          // launcher.
          expect(find.byType(AppGrid), findsOneWidget);
          expect(find.text('item-home'), findsOneWidget);
        });
      }
    }
  });

  group('pages', () {
    testWidgets('no pages at all is an empty grid under the strip', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: const [], items: const []);
      await pumpHome(tester);
      expect(find.byType(TabStrip), findsOneWidget);
      expect(find.byType(AppGrid), findsOneWidget);
      expect(find.byType(AppCell), findsNothing);
      expect(tester.takeException(), isNull);
    });

    testWidgets('the strip centres the group the pager shows', (tester) async {
      setSurface(tester, const Size(600, 800));
      await load();
      await pumpHome(tester);

      final current = find.byKey(TabStrip.tabKey(0));
      double currentDx() => tester.getCenter(current).dx;

      expect((currentDx() - 300).abs(), lessThan(4));
      expect(tester.widget<Text>(current).data, 'home');

      await tester.drag(find.byType(PageView), const Offset(-600, 0));
      await tester.pumpAndSettle();

      expect((currentDx() - 300).abs(), lessThan(4));
      expect(tester.widget<Text>(current).data, 'apps');
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'apps');
    });

    testWidgets('an unchanged page list does not re-anchor the pager', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);

      // Swipe backwards: the user ends on a wrapped copy of the group, not on
      // the canonical page the controller would re-anchor to.
      await tester.drag(find.byType(PageView), const Offset(400, 0));
      await tester.pumpAndSettle();
      expect(find.text('item-apps'), findsOneWidget);
      final before = pagerPage(tester);
      expect(before, lessThan(GroupPagerController.virtualPageCount ~/ 2));

      // A model change that leaves the page list alone must leave the pager
      // where the user put it.
      fake.emit(const CoreEvent.modelChanged());
      await tester.pump();
      await tester.pump();

      expect(pagerPage(tester), before);
      expect(find.text('item-apps'), findsOneWidget);
    });

    testWidgets('a changed page list keeps the group the user was on', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(
        items: [
          makeItem('item-home', 'home'),
          makeItem('item-apps', 'apps'),
          makeItem('item-x', 'x'),
        ],
      );
      await pumpHome(tester);
      expect(find.text('item-home'), findsOneWidget);

      // A group appears before the current one. The user must not silently end
      // up on it.
      fake.current = snapshotOf(
        pageIds: ['x', 'home', 'apps'],
        groups: [
          for (final id in ['x', 'home', 'apps']) makeGroup(id),
        ],
        items: [
          makeItem('item-home', 'home'),
          makeItem('item-apps', 'apps'),
          makeItem('item-x', 'x'),
        ],
      );
      fake.emit(const CoreEvent.modelChanged());
      await tester.pumpAndSettle();

      expect(find.text('item-home'), findsOneWidget);
      expect(find.text('item-x'), findsNothing);
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'home');
    });

    testWidgets(
      'the background layer is under the pages, dimmed by the setting',
      (tester) async {
        setSurface(tester, const Size(400, 800));
        await load(settings: settingsWith(wallpaperAlpha: 60));
        await pumpHome(tester);

        expect(find.byType(BackgroundLayer), findsOneWidget);
        expect(
          tester
              .widget<ColoredBox>(find.byKey(BackgroundLayer.overlayKey))
              .color
              .a,
          closeTo(0.6, 1e-9),
        );
        // No wallpaper bytes exist for Linux: the solid base is what shows.
        expect(find.byKey(BackgroundLayer.baseKey), findsOneWidget);
        // The pages are drawn, and drawn over it.
        expect(find.text('item-home'), findsOneWidget);
      },
    );
  });

  group('the tab strip', () {
    testWidgets('tapping a tab switches to that group', (tester) async {
      setSurface(tester, const Size(600, 800));
      await load();
      await pumpHome(tester);

      await tester.tap(find.byKey(TabStrip.tabKey(1)));
      await tester.pumpAndSettle();

      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'apps');
    });
  });

  group('the system Home button', () {
    testWidgets('opens on the home group even when it is not the first page', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: ['apps', 'home', 'games'], homeGroupId: 'home');
      await pumpHome(tester);

      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'home');
      expect(
        pagerPage(tester),
        GroupPagerController(pageIds: ['apps', 'home', 'games'])
            .virtualIndexFor('home'),
      );
    });

    testWidgets('returns the pager to the home group', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: ['home', 'apps'], homeGroupId: 'home');
      await pumpHome(tester);
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'home');

      await tester.drag(find.byType(PageView), const Offset(-400, 0));
      await tester.pumpAndSettle();
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'apps');

      fake.emit(const CoreEvent.homePressed());
      await tester.pumpAndSettle();

      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'home');
      expect(find.text('item-home'), findsOneWidget);
    });

    testWidgets('falls back to the first page when the Home group is hidden', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      fake.current = snapshotOf(
        pageIds: ['apps'],
        homeGroupId: 'home',
        groups: [
          makeGroup('home', role: GroupRole.home, order: 0, hidden: true),
          makeGroup('apps', role: GroupRole.apps, order: 1),
        ],
        items: [makeItem('item-apps', 'apps')],
      );
      await state.load();
      await pumpHome(tester);

      // The Home group is hidden, so it is not a page: the launcher opens on
      // the first page rather than showing nothing.
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'apps');
      expect(find.text('item-apps'), findsOneWidget);
    });

    testWidgets('goes to the first page when no group holds Home', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: ['home', 'apps']);
      await pumpHome(tester);

      await tester.drag(find.byType(PageView), const Offset(-400, 0));
      await tester.pumpAndSettle();
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'apps');

      fake.emit(const CoreEvent.homePressed());
      await tester.pumpAndSettle();

      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'home');
      expect(tester.takeException(), isNull);
    });
  });

  group('the search, opened by the Home button', () {
    testWidgets('search hides the main foreground and keeps its background', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(
        homeGroupId: 'home',
        settings: settingsWith(wallpaperAlpha: 35),
      );
      await pumpHome(tester);
      fake.emit(const CoreEvent.homePressed());
      await tester.pumpAndSettle();

      expect(find.byType(AppSearch), findsOneWidget);
      expect(find.byType(AppGrid), findsNothing);
      expect(find.text('item-home'), findsNothing);
      expect(find.byType(BackgroundLayer), findsOneWidget);
      expect(
        tester
            .widget<ColoredBox>(find.byKey(BackgroundLayer.overlayKey))
            .color
            .a,
        closeTo(0.35, 1e-9),
      );

      fake.emit(const CoreEvent.homePressed());
      await tester.pumpAndSettle();
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'home');
    });

    testWidgets('leaving the app closes search before returning', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(homeGroupId: 'home');
      await pumpHome(tester);
      fake.emit(const CoreEvent.homePressed());
      await tester.pumpAndSettle();
      expect(find.byType(AppSearch), findsOneWidget);

      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
      await tester.pump();
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pumpAndSettle();
      expect(find.byType(AppSearch), findsNothing);
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'home');

      fake.emit(const CoreEvent.homePressed());
      await tester.pumpAndSettle();
      expect(find.byType(AppSearch), findsOneWidget);
    });

    testWidgets('the Home button on the home group opens the search', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: ['home', 'apps'], homeGroupId: 'home');
      await pumpHome(tester);
      expect(find.byType(AppSearch), findsNothing);

      fake.emit(const CoreEvent.homePressed());
      await tester.pump();
      await tester.pump();

      expect(find.byType(AppSearch), findsOneWidget);
    });

    testWidgets('from another group the first press only goes home', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: ['home', 'apps'], homeGroupId: 'home');
      await pumpHome(tester);
      await tester.drag(find.byType(PageView), const Offset(-400, 0));
      await tester.pumpAndSettle();
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'apps');

      fake.emit(const CoreEvent.homePressed());
      await tester.pumpAndSettle();
      expect(tester.widget<AppGrid>(find.byType(AppGrid)).groupId, 'home');
      expect(find.byType(AppSearch), findsNothing);

      // Now that it is home, a second press opens the search.
      fake.emit(const CoreEvent.homePressed());
      await tester.pump();
      await tester.pump();
      expect(find.byType(AppSearch), findsOneWidget);
    });

    testWidgets('a second press closes the search again', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: ['home', 'apps'], homeGroupId: 'home');
      await pumpHome(tester);

      fake.emit(const CoreEvent.homePressed());
      await tester.pump();
      await tester.pump();
      expect(find.byType(AppSearch), findsOneWidget);

      fake.emit(const CoreEvent.homePressed());
      await tester.pump();
      await tester.pump();
      expect(find.byType(AppSearch), findsNothing);
    });
  });

  group('the system Back button', () {
    testWidgets('closes the open search', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: ['home', 'apps'], homeGroupId: 'home');
      await pumpHome(tester);

      fake.emit(const CoreEvent.homePressed());
      await tester.pump();
      await tester.pump();
      expect(find.byType(AppSearch), findsOneWidget);

      fake.emit(const CoreEvent.backPressed());
      await tester.pump();
      await tester.pump();
      expect(find.byType(AppSearch), findsNothing);
    });

    testWidgets('while the settings screen is open, leaves it and keeps the '
        'search', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: ['home', 'apps'], homeGroupId: 'home');
      await pumpHome(tester);

      fake.emit(const CoreEvent.homePressed());
      await tester.pumpAndSettle();
      expect(find.byType(AppSearch), findsOneWidget);

      await tester.tap(find.byKey(AppSearch.gearKey));
      await tester.pumpAndSettle();
      expect(find.byType(SettingsScreen), findsOneWidget);

      fake.emit(const CoreEvent.backPressed());
      await tester.pumpAndSettle();

      // Back leaves the settings and returns to the panel, which was kept.
      expect(find.byType(SettingsScreen), findsNothing);
      expect(find.byType(AppSearch), findsOneWidget);
    });

    testWidgets('with no search open does nothing', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: ['home', 'apps'], homeGroupId: 'home');
      await pumpHome(tester);
      expect(find.byType(AppSearch), findsNothing);

      fake.emit(const CoreEvent.backPressed());
      await tester.pump();
      await tester.pump();

      // The launcher is still there; Back never opens the search or changes
      // the group.
      expect(find.byType(AppSearch), findsNothing);
      expect(find.byType(AppGrid), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  });

  group('a long press on blank space', () {
    testWidgets('opens the launcher menu', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);

      // A point inside the grid but clear of the one item in the first row.
      final grid = tester.getRect(find.byType(GridView));
      await tester.longPressAt(Offset(grid.left + 30, grid.bottom - 30));
      await tester.pumpAndSettle();

      expect(find.byKey(LauncherMenu.settingsKey), findsOneWidget);
      expect(find.byKey(LauncherMenu.addShortcutKey), findsOneWidget);
      expect(find.byKey(LauncherMenu.editGroupsKey), findsOneWidget);
      expect(find.byKey(LauncherMenu.wallpaperKey), findsOneWidget);
    });

    testWidgets('its settings action opens the settings screen', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);

      final grid = tester.getRect(find.byType(GridView));
      await tester.longPressAt(Offset(grid.left + 30, grid.bottom - 30));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(LauncherMenu.settingsKey));
      await tester.pumpAndSettle();

      expect(find.byType(SettingsScreen), findsOneWidget);
    });

    testWidgets('on an item does not open the menu', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);

      await tester.longPress(find.text('item-home'));
      await tester.pumpAndSettle();

      expect(find.byKey(LauncherMenu.settingsKey), findsNothing);
      expect(tester.takeException(), isNull);
    });
  });

  group('a double tap on blank space', () {
    testWidgets('asks the core to lock the screen', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);

      final grid = tester.getRect(find.byType(GridView));
      final blank = Offset(grid.left + 30, grid.bottom - 30);
      await tester.tapAt(blank);
      await tester.pump(const Duration(milliseconds: 50));
      await tester.tapAt(blank);
      await tester.pumpAndSettle();

      expect(fake.calls, contains('lockScreen'));
    });

    testWidgets('on an item launches it and does not lock', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);

      final item = tester.getCenter(find.text('item-home'));
      await tester.tapAt(item);
      await tester.pump(const Duration(milliseconds: 50));
      await tester.tapAt(item);
      await tester.pump();

      expect(fake.calls, isNot(contains('lockScreen')));
      expect(
        fake.calls.where((call) => call.startsWith('launchApp(')),
        isNotEmpty,
      );
    });
  });

  group('a long press on an item', () {
    testWidgets('opens the app settings popup', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);

      await tester.longPress(find.text('item-home'));
      await tester.pumpAndSettle();

      expect(find.byType(AppSettingsDialog), findsOneWidget);
    });
  });

  group('errors', () {
    testWidgets('a failure the grid would swallow is shown to the user', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);
      captureReports();

      fake.failure = StateError('the app is gone');
      await tester.tap(find.text('item-home'));
      await tester.pump();
      await tester.pump();

      expect(state.error, contains('the app is gone'));
      expect(find.byType(SnackBar), findsOneWidget);
      expect(find.textContaining('the app is gone'), findsOneWidget);
    });

    testWidgets('the same failure is not shown twice', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);
      captureReports();

      fake.failure = StateError('the app is gone');
      await tester.tap(find.text('item-home'));
      await tester.pump();
      await expireMessage(tester);
      expect(find.textContaining('the app is gone'), findsNothing);

      // The same failure again: nothing new to say, so nothing is said.
      await tester.tap(find.text('item-home'));
      await tester.pump();
      expect(state.error, contains('the app is gone'));
      expect(find.textContaining('the app is gone'), findsNothing);
    });

    testWidgets('a failure after a good fetch is shown again', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);
      captureReports();

      fake.failure = StateError('the app is gone');
      await tester.tap(find.text('item-home'));
      await tester.pump();
      await expireMessage(tester);

      // The next fetch succeeds, which clears the error.
      fake.failure = null;
      fake.emit(const CoreEvent.modelChanged());
      await tester.pumpAndSettle();

      fake.failure = StateError('the app is gone');
      await tester.tap(find.text('item-home'));
      await tester.pump();
      expect(find.textContaining('the app is gone'), findsOneWidget);
    });
  });

  group('icon prefetch', () {
    testWidgets('mounting while already paused does not start warming', (
      tester,
    ) async {
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
      addTearDown(
        () => tester.binding.handleAppLifecycleStateChanged(
          AppLifecycleState.resumed,
        ),
      );
      final scheduled = <Future<void> Function()>[];
      final warmer = IconWarmer(schedule: scheduled.add);
      setSurface(tester, const Size(400, 300));
      await load(pageIds: ['home', 'apps', 'games', 'office', 'chat']);
      await pumpHome(tester, warmer: warmer);
      while (scheduled.isNotEmpty) {
        await scheduled.removeAt(0)();
      }
      expect(fake.calls, isNot(contains('iconFor(pkg.item-office,Main,48)')));
    });

    testWidgets('many tiny shortcuts are all warmed', (tester) async {
      final scheduled = <Future<void> Function()>[];
      final warmer = IconWarmer(schedule: scheduled.add);
      setSurface(tester, const Size(400, 300));
      final items = [
        for (var i = 0; i < 17; i++)
          makeItem('shortcut$i', 'chat', iconOverride: 'icon$i.png'),
        for (final group in ['home', 'apps', 'games'])
          launchable('item-$group', group),
      ];
      await load(pageIds: ['home', 'apps', 'games', 'chat'], items: items);
      await pumpHome(tester, warmer: warmer);
      while (scheduled.isNotEmpty) {
        await scheduled.removeAt(0)();
      }
      expect(
        fake.calls.where((call) => call.startsWith('itemIcon(')),
        hasLength(17),
      );
    });

    testWidgets('scrolling pauses speculative loads until it ends', (
      tester,
    ) async {
      final scheduled = <Future<void> Function()>[];
      final warmer = IconWarmer(schedule: scheduled.add);
      setSurface(tester, const Size(400, 300));
      await load(pageIds: ['home', 'apps', 'games', 'office', 'chat']);
      await pumpHome(tester, warmer: warmer);
      final gesture = await tester.startGesture(
        tester.getCenter(find.byType(GroupPager)),
      );
      await gesture.moveBy(const Offset(-120, 0));
      await tester.pump();
      while (scheduled.isNotEmpty) {
        await scheduled.removeAt(0)();
      }
      expect(fake.calls, isNot(contains('iconFor(pkg.item-office,Main,48)')));
      await gesture.up();
      await tester.pumpAndSettle();
      while (scheduled.isNotEmpty) {
        await scheduled.removeAt(0)();
      }
      expect(fake.calls, contains('iconFor(pkg.item-office,Main,48)'));
    });

    testWidgets('memory pressure stops an in-flight warm pass', (tester) async {
      final scheduled = <Future<void> Function()>[];
      final decoding = Completer<void>();
      final warmer = IconWarmer(
        schedule: scheduled.add,
        warm: (_) => decoding.future,
      );
      setSurface(tester, const Size(400, 300));
      await load(pageIds: ['home', 'apps', 'games', 'office', 'chat']);
      await pumpHome(tester, warmer: warmer);

      final first = scheduled.removeAt(0)();
      await tester.pump();
      final loaded = fake.iconCalls;
      tester.binding.handleMemoryPressure();
      decoding.complete();
      await first;
      while (scheduled.isNotEmpty) {
        await scheduled.removeAt(0)();
      }
      expect(fake.iconCalls, loaded);
    });

    testWidgets(
      'background, search and covered routes pause speculative loads',
      (tester) async {
        final scheduled = <Future<void> Function()>[];
        final warmer = IconWarmer(schedule: scheduled.add);
        setSurface(tester, const Size(400, 300));
        await load(pageIds: ['home', 'apps', 'games', 'office', 'chat']);
        await pumpHome(tester, warmer: warmer);
        final initial = fake.iconCalls;
        tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
        await scheduled.removeAt(0)();
        expect(fake.iconCalls, initial);
        tester.binding.handleAppLifecycleStateChanged(
          AppLifecycleState.resumed,
        );
        final navigator = tester.state<NavigatorState>(find.byType(Navigator));
        unawaited(
          navigator.push(
            MaterialPageRoute<void>(
              builder: (_) => const Scaffold(body: Text('cover')),
            ),
          ),
        );
        await tester.pumpAndSettle();
        await scheduled.removeAt(0)();
        expect(fake.iconCalls, initial);
        navigator.pop();
        await tester.pumpAndSettle();
        fake.emit(const CoreEvent.homePressed());
        await tester.pumpAndSettle();
        await scheduled.removeAt(0)();
        expect(fake.calls, isNot(contains('iconFor(pkg.item-office,Main,48)')));
        fake.emit(const CoreEvent.backPressed());
        await tester.pumpAndSettle();
        while (scheduled.isNotEmpty) {
          await scheduled.removeAt(0)();
        }
        expect(fake.calls, contains('iconFor(pkg.item-office,Main,48)'));
      },
    );

    Future<void> drainIdle(WidgetTester tester) async {
      // Widget-test fake time does not run the real scheduler's idle event loop.
      for (var i = 0; i < 160; i++) {
        tester.binding.handleEventLoopCallback();
        await tester.pump();
      }
    }

    testWidgets(
      'idle warming reaches non-neighbouring pages without Home-triggered refills',
      (tester) async {
        setSurface(tester, const Size(400, 300));
        await load(pageIds: ['home', 'apps', 'games', 'office', 'chat']);
        await pumpHome(tester);
        await drainIdle(tester);
        expect(fake.calls, contains('iconFor(pkg.item-games,Main,48)'));
        expect(fake.calls, contains('iconFor(pkg.item-office,Main,48)'));
        final before = fake.iconCalls;
        tester.binding.handleMemoryPressure();
        fake.emit(const CoreEvent.backPressed());
        await tester.pump();
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 30)),
        );
        expect(fake.iconCalls, before);
      },
    );

    testWidgets('opening a page warms every icon on it, not just the visible '
        'cells', (tester) async {
      setSurface(tester, const Size(400, 300));
      // Far more items than the lazy grid builds for this small surface.
      final items = [for (var i = 0; i < 120; i++) launchable('a$i', 'home')];
      await load(items: items);
      expect(fake.iconCalls, 0);
      await pumpHome(tester);
      await tester.pumpAndSettle();
      await drainIdle(tester);
      expect(fake.iconCalls, 120);
    });

    testWidgets('opening a page warms its neighbouring pages too', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 300));
      final items = [
        for (final group in ['home', 'apps', 'games'])
          for (var i = 0; i < 10; i++) launchable('$group-$i', group),
      ];
      await load(pageIds: ['home', 'apps', 'games'], items: items);
      await pumpHome(tester);
      await tester.pumpAndSettle();
      await drainIdle(tester);
      expect(fake.iconCalls, 30);
    });

    testWidgets('hidden icons are not warmed', (tester) async {
      setSurface(tester, const Size(400, 300));
      final items = [for (var i = 0; i < 40; i++) launchable('a$i', 'home')];
      await load(items: items, settings: settingsWith(iconsVisible: false));
      await pumpHome(tester);
      await tester.pumpAndSettle();
      expect(fake.iconCalls, 0);
    });
  });

  group('raster generations', () {
    testWidgets(
      'tabs and grid share one generation that paging does not churn',
      (tester) async {
        setSurface(tester, const Size(400, 800));
        TileRasterCache.store.clear();
        addTearDown(TileRasterCache.reset);
        await load(
          pageIds: ['home', 'apps', 'games'],
          settings: settingsWith(labelShadowBlur: 5),
          items: [
            for (final id in ['home', 'apps', 'games'])
              launchable('item-$id', id),
          ],
        );
        await pumpHome(tester);
        await tester.pumpAndSettle();

        Object generation() => tester
            .widget<RasterCacheScope>(find.byType(RasterCacheScope))
            .generation;
        final first = generation();
        expect(TileRasterCache.store.currentGeneration, first);

        // Paging and rebuilding must not create a new appearance generation.
        await tester.tap(find.byKey(TabStrip.tapKey(0)));
        await tester.pumpAndSettle();
        expect(generation(), first);

        final seen = <Object>[first];
        for (final blur in [6, 7]) {
          await load(settings: settingsWith(labelShadowBlur: blur));
          await tester.pump();
          await tester.pumpAndSettle();
          expect(generation(), isNot(seen.last));
          expect(TileRasterCache.store.currentGeneration, generation());
          seen.add(generation());
        }
        await load(settings: settingsWith(labelShadowBlur: 5));
        await tester.pump();
        await tester.pumpAndSettle();
        // Returning to an existing appearance reuses its generation.
        expect(generation(), first);
        expect(TileRasterCache.store.currentGeneration, first);

        final store = TileRasterCache.store;
        final probes = <Object, RasterCacheKey>{};
        for (var i = 0; i < seen.length; i++) {
          final key = RasterCacheKey(
            generation: seen[i],
            kind: 'probe',
            owner: 'probe:$i',
            pixels: i,
          );
          store
              .insertAndAcquire(key, fixtures.image(2, 2), 1, const Size(2, 2))
              .release();
          probes[seen[i]] = key;
        }

        // A fourth distinct appearance bounds historical retention to three.
        await load(settings: settingsWith(labelShadowBlur: 8));
        await tester.pump();
        await tester.pumpAndSettle();
        final fourth = generation();
        expect(fourth, isNot(first));
        for (final earlier in seen.sublist(1)) {
          expect(fourth, isNot(earlier));
        }
        expect(store.contains(probes[seen[1]]!), isFalse);
        expect(store.contains(probes[seen[0]]!), isTrue);
        expect(store.contains(probes[seen[2]]!), isTrue);
      },
    );
  });

  group('memory pressure', () {
    late RasterImageStore store;

    setUp(() {
      store = RasterImageStore();
      addTearDown(store.clear);
    });

    testWidgets('keeps displayed pixels, route and settings; no warming', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(
        pageIds: ['home', 'apps'],
        homeGroupId: 'home',
        settings: settingsWith(labelShadow: true),
      );
      await pumpHome(tester, rasterStore: store);
      await tester.pumpAndSettle();

      final generation = store.currentGeneration ?? 0;
      final pinned = store.insertAndAcquire(
        RasterCacheKey(
          generation: generation,
          kind: 'tile',
          owner: 'item:item-home',
          pixels: 'pinned',
        ),
        fixtures.image(4, 4),
        1,
        const Size(4, 4),
      );
      final group = tester
          .widget<TabStrip>(find.byType(TabStrip))
          .currentGroupId;
      final calls = fake.iconCalls;

      tester.binding.handleMemoryPressure();
      await tester.pumpAndSettle();

      expect(store.statistics.pinnedBytes, 64);
      expect(store.statistics.retainedBytes, 64);
      expect(
        tester.widget<TabStrip>(find.byType(TabStrip)).currentGroupId,
        group,
      );
      expect(state.snapshot!.settings.labelShadow, isTrue);
      expect(fake.iconCalls, calls);
      expect(tester.takeException(), isNull);

      pinned.release();
      expect(store.contains(pinned.key), isFalse);
      expect(store.statistics.liveBytes, 0);

      // Repeated pressure stays a no-op.
      tester.binding.handleMemoryPressure();
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });

    testWidgets('pressure in search and after resume does not repopulate', (
      tester,
    ) async {
      setSurface(tester, const Size(400, 800));
      await load(homeGroupId: 'home');
      await pumpHome(tester, rasterStore: store);
      fake.emit(const CoreEvent.homePressed());
      await tester.pumpAndSettle();
      expect(find.byType(AppSearch), findsOneWidget);

      tester.binding.handleMemoryPressure();
      for (final lifecycle in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(lifecycle);
      }
      await tester.pumpAndSettle();

      // Pausing closes the search; resuming rebuilds only what is required,
      // and the pressure trim left no speculative work queued.
      expect(find.byType(AppSearch), findsNothing);
      expect(store.statistics.liveBytes, 0);
      expect(tester.takeException(), isNull);
    });

    testWidgets('disposal releases every raster lease', (tester) async {
      setSurface(tester, const Size(400, 800));
      TileRasterCache.mode = TileRasterMode.combined;
      addTearDown(TileRasterCache.reset);
      await load(
        pageIds: ['home'],
        items: [for (var i = 0; i < 8; i++) launchable('h$i', 'home')],
      );
      await pumpHome(tester, rasterStore: store);
      await tester.pumpAndSettle();
      expect(store.statistics.liveBytes, greaterThan(0));

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
      expect(store.statistics.pinnedBytes, 0);
      expect(store.statistics.retiredLiveBytes, 0);
    });
  });
}
