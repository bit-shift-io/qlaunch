import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
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
      fake.emit(const CoreEvent.homePressed());
      fake.emit(const CoreEvent.backPressed());
      await pumpEventQueue();

      expect(fake.calls, isEmpty);
      expect(state.error, isNull);
      expect(notifications, 1);
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
        fake.emit(const CoreEvent.modelChanged());
        await pumpEventQueue();
        expect(fake.held, hasLength(2));
        final before = notifications;

        fake.held[1].complete();
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
      fake.emit(const CoreEvent.modelChanged());
      await pumpEventQueue();

      fake.held[1].complete();
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
}
