import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/api/api.dart';
import 'package:qlaunch/src/rust/model/icon.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/usage.dart';
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/state/launcher_state.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  test(
    'app revisions preserve unchanged providers and invalidate updated apps',
    () async {
      final backend = FakeBackend();
      final launcher = LauncherState(backend);
      const a = AppRef(package: 'a', activity: 'Main');
      const b = AppRef(package: 'b', activity: 'Main');
      backend.icons[(package: 'a', activity: 'Main')] = IconImage(
        format: IconFormat.rgba,
        width: 1,
        height: 1,
        bytes: Uint8List(4),
      );
      backend.icons[(package: 'b', activity: 'Main')] = IconImage(
        format: IconFormat.rgba,
        width: 1,
        height: 1,
        bytes: Uint8List(4),
      );
      backend.current = snapshotOf(
        iconRevisions: [
          const IconRevision(app: a, revision: 0),
          const IconRevision(app: b, revision: 0),
        ],
      );
      await launcher.load();
      final firstA = await launcher.iconFor('a', 'Main', 48);
      final firstB = await launcher.iconFor('b', 'Main', 48);
      backend.current = snapshotOf(
        iconRevisions: [
          const IconRevision(app: a, revision: 1),
          const IconRevision(app: b, revision: 0),
        ],
      );
      backend.emit(const CoreEvent.appsChanged());
      await Future<void>.delayed(Duration.zero);
      expect(await launcher.iconFor('b', 'Main', 48), same(firstB));
      expect(await launcher.iconFor('a', 'Main', 48), isNot(same(firstA)));
      expect(backend.iconCalls, 3);
      launcher.dispose();
      await backend.close();
    },
  );
  late FakeBackend fake;
  late LauncherState state;
  late int notifications;
  late List<FlutterErrorDetails> reported;
  var disposed = false;

  void disposeState() {
    disposed = true;
    state.dispose();
  }

  setUp(() {
    fake = FakeBackend();
    state = LauncherState(fake);
    notifications = 0;
    disposed = false;
    state.addListener(() => notifications++);
    reported = [];
    final previousOnError = FlutterError.onError;
    FlutterError.onError = reported.add;
    addTearDown(() => FlutterError.onError = previousOnError);
  });

  tearDown(() async {
    if (!disposed) state.dispose();
    await fake.close();
  });

  group('before load()', () {
    test('every accessor answers empty or null and nothing throws', () {
      expect(state.snapshot, isNull);
      expect(state.pageIds, isEmpty);
      expect(state.groups, isEmpty);
      expect(state.itemsIn('apps'), isEmpty);
      expect(state.homeGroup, isNull);
      expect(state.error, isNull);
    });
  });

  group('load()', () {
    test('makes the snapshot available and notifies exactly once', () async {
      fake.current = snapshotOf(groups: [makeGroup('apps')]);

      await state.load();

      expect(state.snapshot, same(fake.current));
      expect(state.groups.map((g) => g.id), ['apps']);
      expect(notifications, 1);
    });

    test('passes configRootOverride through to init', () async {
      await state.load(configRootOverride: '/tmp/cfg');

      expect(fake.lastConfigRootOverride, '/tmp/cfg');
    });

    test('never throws: a failing init becomes error state', () async {
      fake.failure = StateError('no config root');

      await state.load();

      expect(state.snapshot, isNull);
      expect(state.error, contains('no config root'));
      expect(notifications, 1);
      expect(reported, hasLength(1));
    });

    test('subscribes to events before it fetches', () async {
      await state.load();

      expect(fake.calls.indexOf('events'), isNonNegative);
      expect(
        fake.calls.indexOf('events'),
        lessThan(fake.calls.indexOf('init')),
      );
    });

    test('a second load() does not subscribe twice', () async {
      await state.load();
      await state.load();
      fake.calls.clear();

      fake.emit(const CoreEvent.modelChanged());
      await pumpEventQueue();

      expect(fake.snapshotCalls, 1);
    });
  });

  group('events', () {
    test(
      'a burst while fetching has one active snapshot and one follow-up',
      () async {
        await state.load();
        fake.holdSnapshots = true;
        fake.emit(const CoreEvent.modelChanged());
        await pumpEventQueue();
        for (var i = 0; i < 20; i++) {
          fake.emit(const CoreEvent.appsChanged());
        }
        await pumpEventQueue();
        expect(fake.snapshotCalls, 1);
        fake.current = snapshotOf(groups: [makeGroup('external')]);
        fake.held.first.complete();
        await pumpEventQueue();
        expect(fake.snapshotCalls, 2);
        fake.held.last.complete();
        await pumpEventQueue();
        expect(state.groups.single.id, 'external');
        expect(fake.snapshotCalls, 2);
      },
    );

    test(
      'ModelChanged re-fetches, adopts the new snapshot and notifies',
      () async {
        await state.load();
        fake.current = snapshotOf(groups: [makeGroup('games')]);

        fake.emit(const CoreEvent.modelChanged());
        await pumpEventQueue();

        expect(fake.snapshotCalls, 1);
        expect(state.groups.map((g) => g.id), ['games']);
        expect(notifications, 2);
      },
    );

    test('AppsChanged re-fetches too', () async {
      await state.load();
      fake.current = snapshotOf(groups: [makeGroup('games')]);

      fake.emit(const CoreEvent.appsChanged());
      await pumpEventQueue();

      expect(fake.snapshotCalls, 1);
      expect(state.groups.map((g) => g.id), ['games']);
      expect(notifications, 2);
    });

    test('Error surfaces as error state, not an exception', () async {
      await state.load();

      fake.emit(const CoreEvent.error('platform exploded'));
      await pumpEventQueue();

      expect(state.error, 'platform exploded');
      expect(fake.snapshotCalls, 0);
      expect(notifications, 2);
    });

    test('Error clears on the next successful fetch', () async {
      await state.load();
      fake.emit(const CoreEvent.error('platform exploded'));
      await pumpEventQueue();
      expect(state.error, isNotNull);

      fake.emit(const CoreEvent.modelChanged());
      await pumpEventQueue();

      expect(state.error, isNull);
    });

    test(
      'a failing re-fetch sets the error, notifies and keeps the snapshot',
      () async {
        fake.current = snapshotOf(groups: [makeGroup('apps')]);
        await state.load();
        final adopted = state.snapshot;
        fake.failure = StateError('core is gone');

        fake.emit(const CoreEvent.modelChanged());
        await pumpEventQueue();

        expect(state.error, contains('core is gone'));
        expect(state.snapshot, same(adopted));
        expect(notifications, 2);
        expect(reported, hasLength(1));
        expect(reported.single.exception, isA<StateError>());
        expect(reported.single.stack, isNotNull);
      },
    );

    test('a stream error sets the error, notifies and is reported', () async {
      await state.load();

      fake.emitError(StateError('stream broke'));
      await pumpEventQueue();

      expect(state.error, contains('stream broke'));
      expect(notifications, 2);
      expect(reported, hasLength(1));
    });

    test('events with no v1 behaviour change nothing', () async {
      await state.load();
      fake.calls.clear();

      fake.emit(const CoreEvent.shortcutCreated(itemId: 'x'));
      fake.emit(const CoreEvent.shortcutCancelled());
      fake.emit(const CoreEvent.wallpaperChanged());
      await pumpEventQueue();

      expect(fake.calls, isEmpty);
      expect(state.error, isNull);
      expect(state.homeRequests, 0);
      expect(state.backRequests, 0);
      expect(notifications, 1);
    });

    test('the Back button bumps backRequests and notifies', () async {
      await state.load();
      fake.calls.clear();
      final before = notifications;

      fake.emit(const CoreEvent.backPressed());
      await pumpEventQueue();

      expect(state.backRequests, 1);
      expect(notifications, before + 1);
      expect(fake.calls, isEmpty);
    });

    test('the Home button bumps homeRequests and notifies', () async {
      await state.load();
      fake.calls.clear();
      final before = notifications;

      fake.emit(const CoreEvent.homePressed());
      await pumpEventQueue();

      expect(state.homeRequests, 1);
      expect(notifications, before + 1);
      expect(fake.calls, isEmpty);
    });

    test('every Home press is a new request', () async {
      await state.load();

      fake.emit(const CoreEvent.homePressed());
      fake.emit(const CoreEvent.homePressed());
      await pumpEventQueue();

      expect(state.homeRequests, 2);
    });
  });

  group('chooseWallpaper', () {
    test('reaches the backend', () async {
      await state.load();
      fake.calls.clear();

      await state.chooseWallpaper();

      expect(fake.calls, contains('chooseWallpaper'));
      expect(state.error, isNull);
    });

    test('a failure is state, not a throw', () async {
      await state.load();
      fake.failure = StateError('no chooser');

      await state.chooseWallpaper();

      expect(state.error, contains('no chooser'));
      expect(reported, hasLength(1));
    });
  });

  group('usage queries', () {
    test('pass the limit through and answer with the backend result', () async {
      await state.load();
      fake.frequentAppsResults = (limit) => [
        UsageHit(
          app: AppRef(package: 'p', activity: 'Main'),
          label: 'P',
        ),
      ];

      final hits = await state.frequentApps(24);

      expect(hits.single.label, 'P');
      expect(fake.usageQueries, ['frequent(24)']);
    });

    test('a failure yields no hits and is reported, not thrown', () async {
      await state.load();
      fake.failure = StateError('boom');

      expect(await state.recentApps(24), isEmpty);
      expect(reported, hasLength(1));
      expect(state.error, isNull, reason: 'search-like failures are not state');
    });
  });

  group('itemsIn', () {
    test(
      'returns only that group\'s items, in exactly the snapshot order',
      () async {
        fake.current = snapshotOf(
          groups: [makeGroup('apps'), makeGroup('games')],
          items: [
            makeItem('zebra', 'apps', order: 2),
            makeItem('g2', 'games', order: 1),
            makeItem('mango', 'apps', order: 0),
            makeItem('mango', 'apps', order: 0),
            makeItem('apple', 'apps', order: 1),
            makeItem('g1', 'games', order: 0),
          ],
        );
        await state.load();

        expect(state.itemsIn('apps').map((i) => i.id), [
          'zebra',
          'mango',
          'mango',
          'apple',
        ]);
        expect(state.itemsIn('games').map((i) => i.id), ['g2', 'g1']);
        expect(state.itemsIn('nowhere'), isEmpty);
      },
    );
  });

  group('pageIds', () {
    test('are the snapshot\'s, not recomputed from groups', () async {
      fake.current = snapshotOf(
        groups: [
          makeGroup('a', order: 0),
          makeGroup('b', order: 1),
          makeGroup('c', order: 2),
        ],
        pageIds: ['c', 'ghost', 'a'],
      );
      await state.load();

      expect(state.pageIds, ['c', 'ghost', 'a']);
    });
  });

  group('homeGroup', () {
    test('resolves through homeGroupId', () async {
      fake.current = snapshotOf(
        homeGroupId: 'h',
        groups: [makeGroup('apps'), makeGroup('h')],
      );
      await state.load();

      expect(state.homeGroup?.id, 'h');
    });

    test('does not go looking for a group with the home role', () async {
      fake.current = snapshotOf(
        homeGroupId: null,
        groups: [makeGroup('apps', role: GroupRole.home)],
      );
      await state.load();

      expect(state.homeGroup, isNull);
    });

    test('is null when the id names nothing', () async {
      fake.current = snapshotOf(
        homeGroupId: 'missing',
        groups: [makeGroup('apps')],
      );
      await state.load();

      expect(state.homeGroup, isNull);
    });
  });

  group('dispose()', () {
    test('an event after dispose throws nothing and does not notify', () async {
      await state.load();
      final before = notifications;

      disposeState();
      fake.emit(const CoreEvent.modelChanged());
      fake.emit(const CoreEvent.error('late'));
      await pumpEventQueue();

      expect(notifications, before);
    });

    test('a re-fetch in flight at dispose does not notify', () async {
      await state.load();
      final before = notifications;
      fake.holdSnapshots = true;

      fake.emit(const CoreEvent.modelChanged());
      await pumpEventQueue();
      expect(fake.held, hasLength(1));

      disposeState();
      fake.held.single.complete();
      await pumpEventQueue();

      expect(notifications, before);
    });

    test(
      'completes even when cancelling the subscription never does',
      () async {
        final hanging = FakeBackend(hangOnCancel: true);
        final hangingState = LauncherState(hanging);
        await hangingState.load();

        hangingState.dispose();

        // super.dispose() ran synchronously: a disposed notifier rejects use.
        expect(() => hangingState.addListener(() {}), throwsFlutterError);
      },
    );
  });

  group('shortcut icons', () {
    test(
      'a reload during a held write cannot survive the replacement',
      () async {
        fake.current = snapshotOf(
          pageIds: ['home'],
          groups: [makeGroup('home')],
          items: [makeItem('shortcut', 'home', iconOverride: 'icon.png')],
        );
        await state.load();
        fake.itemIcons['shortcut'] = [1, 2, 3];
        final first = await state.itemIcon(
          'shortcut',
          reference: 'icon.png',
          sizePx: 48,
        );
        expect(first, isNotNull);

        fake.holdSnapshots = true;
        final write = state.setItemIcon('shortcut', [9, 9, 9]);
        await pumpEventQueue();
        expect(fake.held, hasLength(1));

        // A rebuild during the held write loads and caches the old bytes.
        final during = await state.itemIcon(
          'shortcut',
          reference: 'icon.png',
          sizePx: 48,
        );
        expect(state.cachedItemIcon('shortcut', 'icon.png', 48), same(during));

        fake.held.single.complete();
        await write;
        expect(state.cachedItemIcon('shortcut', 'icon.png', 48), isNull);
      },
    );
  });

  group('overlapping fetches', () {
    test(
      'a newer snapshot is not overwritten by an older fetch finishing later',
      () async {
        await state.load();
        fake.holdSnapshots = true;
        fake.current = snapshotOf(groups: [makeGroup('old')]);
        fake.emit(const CoreEvent.modelChanged());
        await pumpEventQueue();
        fake.current = snapshotOf(groups: [makeGroup('new')]);
        final newer = state.updateSettings(const SettingsPatch(iconSize: 48));
        await pumpEventQueue();
        expect(fake.held, hasLength(2));
        final before = notifications;

        fake.held[1].complete();
        await newer;
        await pumpEventQueue();
        expect(state.groups.map((g) => g.id), ['new']);

        fake.held[0].complete();
        await pumpEventQueue();
        expect(state.groups.map((g) => g.id), ['new']);
        expect(notifications, before + 1);
      },
    );

    test('an older failing fetch finishing after a newer one adopted sets no error', () async {
      await state.load();
      fake.holdSnapshots = true;
      fake.failure = StateError('stale failure');
      fake.emit(const CoreEvent.modelChanged());
      await pumpEventQueue();
      fake.failure = null;
      fake.current = snapshotOf(groups: [makeGroup('new')]);
      final newer = state.updateSettings(const SettingsPatch(iconSize: 48));
      await pumpEventQueue();

      fake.held[1].complete();
      await newer;
      fake.held[0].complete();
      await pumpEventQueue();

      expect(state.error, isNull);
      expect(state.groups.map((g) => g.id), ['new']);
      expect(reported, isEmpty);
    });

    test('init is still adopted when a newer fetch failed first', () async {
      fake.holdSnapshots = true;
      fake.current = snapshotOf(groups: [makeGroup('loaded')]);
      final loading = state.load();
      await pumpEventQueue();
      fake.failure = StateError('early failure');
      fake.emit(const CoreEvent.modelChanged());
      await pumpEventQueue();

      fake.held[1].complete();
      await pumpEventQueue();
      expect(state.error, contains('early failure'));

      fake.held[0].complete();
      await loading;

      expect(state.groups.map((g) => g.id), ['loaded']);
      expect(state.error, isNull);
    });

    test('a late init does not overwrite a newer event fetch', () async {
      fake.holdSnapshots = true;
      fake.current = snapshotOf(groups: [makeGroup('old')]);
      final loading = state.load();
      await pumpEventQueue();
      fake.current = snapshotOf(groups: [makeGroup('new')]);
      fake.emit(const CoreEvent.modelChanged());
      await pumpEventQueue();

      fake.held[1].complete();
      await pumpEventQueue();
      fake.held[0].complete();
      await loading;

      expect(state.groups.map((g) => g.id), ['new']);
    });
  });

  group('updateSettings', () {
    test('adopts the snapshot the core returns', () async {
      await state.load();
      fake.current = snapshotOf(groups: [makeGroup('kept')]);
      await state.updateSettings(const SettingsPatch(iconSize: 100));

      expect(fake.settingsPatches, [const SettingsPatch(iconSize: 100)]);
      expect(state.groups.map((g) => g.id), ['kept']);
      // What is shown afterwards is the returned snapshot's setting, not the
      // value that was asked for: the fake's snapshot still has 48.
      expect(state.snapshot!.settings.iconSize, testSettings.iconSize);
      expect(state.error, isNull);
      expect(notifications, greaterThan(0));
    });

    test('a failure is state and a report, never a throw', () async {
      await state.load();
      fake.failure = StateError('read-only filesystem');
      await state.updateSettings(const SettingsPatch(layout: LayoutStyle.list));

      expect(state.error, contains('read-only filesystem'));
      expect(reported, hasLength(1));
      expect(reported.single.exception, isA<StateError>());
    });
  });

  group('mutations', () {
    test('renameItem forwards the label and adopts the snapshot', () async {
      fake.current = snapshotOf(groups: [makeGroup('apps')]);
      await state.load();

      await state.renameItem('i1', 'Firefox');

      expect(fake.calls, contains('renameItem(i1,Firefox)'));
    });

    test('moveItem forwards the destination and order', () async {
      fake.current = snapshotOf(groups: [makeGroup('apps')]);
      await state.load();

      await state.moveItem('i1', 'games', 3);

      expect(fake.calls, contains('moveItem(i1,games,3)'));
    });

    test('appsGroup is the group holding the Apps role', () async {
      fake.current = snapshotOf(
        groups: [
          makeGroup('apps', role: GroupRole.apps),
          makeGroup('games', role: GroupRole.games),
        ],
      );
      await state.load();

      expect(state.appsGroup?.id, 'apps');
    });

    test('pages are the page groups in page order', () async {
      fake.current = snapshotOf(
        pageIds: ['home', 'apps'],
        groups: [makeGroup('apps'), makeGroup('home')],
      );
      await state.load();

      expect(state.pages.map((g) => g.id), ['home', 'apps']);
    });

    test('uninstallApp reports a failure rather than throwing', () async {
      await state.load();
      fake.failure = StateError('gone');

      await state.uninstallApp('com.example');

      expect(state.error, contains('gone'));
    });
  });
}
