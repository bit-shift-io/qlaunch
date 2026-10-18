import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/api/api.dart';
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';

import 'fake_backend.dart';

Group makeGroup(String id, {GroupRole? role, int order = 0}) =>
    Group(id: id, label: id, role: role, order: order);

Item makeItem(String id, String groupId, {int order = 0}) => Item(
  id: id,
  groupId: groupId,
  label: id,
  labelOverridden: false,
  uninstalled: false,
  order: order,
);

ModelSnapshot snapshotOf(
  FakeBackend fake, {
  String rootGroupId = 'root',
  String? homeGroupId,
  List<String> pageIds = const [],
  List<Group> groups = const [],
  List<Item> items = const [],
}) => ModelSnapshot(
  rootGroupId: rootGroupId,
  homeGroupId: homeGroupId,
  pageIds: pageIds,
  groups: groups,
  items: items,
  settings: fake.current.settings,
  warnings: const [],
);

void main() {
  late FakeBackend fake;
  late LauncherState state;
  late int notifications;
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
      fake.current = snapshotOf(fake, groups: [makeGroup('apps')]);

      await state.load();

      expect(state.snapshot, same(fake.current));
      expect(state.groups.map((g) => g.id), ['apps']);
      expect(notifications, 1);
    });

    test('passes configRootOverride through to init', () async {
      await state.load(configRootOverride: '/tmp/cfg');

      expect(fake.lastConfigRootOverride, '/tmp/cfg');
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
        fake.current = snapshotOf(fake, groups: [makeGroup('games')]);

        fake.emit(const CoreEvent.modelChanged());
        await pumpEventQueue();

        expect(fake.snapshotCalls, 1);
        expect(state.groups.map((g) => g.id), ['games']);
        expect(notifications, 2);
      },
    );

    test('AppsChanged re-fetches too', () async {
      await state.load();
      fake.current = snapshotOf(fake, groups: [makeGroup('games')]);

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
      'a failing re-fetch surfaces as error state, not an exception',
      () async {
        await state.load();
        fake.failure = StateError('core is gone');

        fake.emit(const CoreEvent.modelChanged());
        await pumpEventQueue();

        expect(state.error, contains('core is gone'));
      },
    );

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
          fake,
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
        fake,
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
        fake,
        homeGroupId: 'h',
        groups: [makeGroup('apps'), makeGroup('h')],
      );
      await state.load();

      expect(state.homeGroup?.id, 'h');
    });

    test('does not go looking for a group with the home role', () async {
      fake.current = snapshotOf(
        fake,
        homeGroupId: null,
        groups: [makeGroup('apps', role: GroupRole.home)],
      );
      await state.load();

      expect(state.homeGroup, isNull);
    });

    test('is null when the id names nothing', () async {
      fake.current = snapshotOf(
        fake,
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
      final gate = Completer<void>();
      fake.snapshotGate = gate.future;

      fake.emit(const CoreEvent.modelChanged());
      await pumpEventQueue();
      expect(fake.snapshotCalls, 1);

      disposeState();
      gate.complete();
      await pumpEventQueue();

      expect(notifications, before);
    });

    test('does not wait for the stream subscription to cancel', () async {
      await state.load();

      // Completing at all, with no event arriving, is the assertion.
      disposeState();
    });
  });
}
