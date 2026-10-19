import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cell.dart';
import 'package:qlaunch/src/ui/app_grid.dart';
import 'package:qlaunch/src/ui/app_search.dart';
import 'package:qlaunch/src/ui/app_settings_dialog.dart';
import 'package:qlaunch/src/ui/background_layer.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/home_screen.dart';
import 'package:qlaunch/src/ui/page_pips.dart';
import 'package:qlaunch/src/ui/settings_screen.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

import 'fake_backend.dart';
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

  Future<void> pumpHome(WidgetTester tester) async {
    await tester.pumpWidget(MaterialApp(home: HomeScreen(state: state)));
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

  group('page pips', () {
    AnimatedOpacity pipsFade(WidgetTester tester) => tester.widget<AnimatedOpacity>(
      find.ancestor(
        of: find.byType(PagePips),
        matching: find.byType(AnimatedOpacity),
      ),
    );

    testWidgets('fade in when the tabs cannot all fit', (tester) async {
      setSurface(tester, const Size(300, 600));
      await load(pageIds: [for (var i = 0; i < 8; i++) 'group$i']);
      await pumpHome(tester);
      await tester.pumpAndSettle();

      expect(find.byType(PagePips), findsOneWidget);
      expect(pipsFade(tester).opacity, 1);
    });

    testWidgets('stay hidden when every tab fits', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load(pageIds: const ['home', 'apps']);
      await pumpHome(tester);
      await tester.pumpAndSettle();

      // Still in the tree, only transparent, so it can fade rather than pop.
      expect(find.byType(PagePips), findsOneWidget);
      expect(pipsFade(tester).opacity, 0);
    });

    testWidgets('fade out again when the window widens enough', (tester) async {
      setSurface(tester, const Size(300, 600));
      await load(pageIds: [for (var i = 0; i < 8; i++) 'group$i']);
      await pumpHome(tester);
      await tester.pumpAndSettle();
      expect(pipsFade(tester).opacity, 1);

      // Same portrait orientation, just wider, so only the strip's own
      // re-check can bring the pips back down.
      tester.view.physicalSize = const Size(1200, 600);
      await tester.pumpAndSettle();

      expect(pipsFade(tester).opacity, 0);
    });

    testWidgets('draw one dot per page', (tester) async {
      setSurface(tester, const Size(300, 600));
      await load(pageIds: [for (var i = 0; i < 8; i++) 'group$i']);
      await pumpHome(tester);
      await tester.pumpAndSettle();

      for (var i = 0; i < 8; i++) {
        expect(find.byKey(PagePips.dotKey(i)), findsOneWidget);
      }
    });
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

    testWidgets('the strip highlights the group the pager shows', (
      tester,
    ) async {
      setSurface(tester, const Size(600, 800));
      await load();
      await pumpHome(tester);

      final indicator = find.byKey(TabStrip.indicatorKey);
      double indicatorDx() => tester.getCenter(indicator).dx;
      double labelDx(String label) => tester.getCenter(find.text(label)).dx;

      expect(indicator, findsOneWidget);
      expect((indicatorDx() - labelDx('home')).abs(), lessThan(4));

      await tester.drag(find.byType(PageView), const Offset(-600, 0));
      await tester.pumpAndSettle();

      expect((indicatorDx() - labelDx('apps')).abs(), lessThan(4));
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
        GroupPagerController(pageIds: ['apps', 'home', 'games']).virtualIndexFor(
          'home',
        ),
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
    testWidgets('opens the settings screen', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);

      // A point inside the grid but clear of the one item in the first row.
      final grid = tester.getRect(find.byType(GridView));
      await tester.longPressAt(Offset(grid.left + 30, grid.bottom - 30));
      await tester.pumpAndSettle();

      expect(find.byType(SettingsScreen), findsOneWidget);
    });

    testWidgets('on an item does not open the settings screen', (tester) async {
      setSurface(tester, const Size(400, 800));
      await load();
      await pumpHome(tester);

      await tester.longPress(find.text('item-home'));
      await tester.pumpAndSettle();

      expect(find.byType(SettingsScreen), findsNothing);
      expect(tester.takeException(), isNull);
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
}
