import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/raster_image_store.dart';

import 'raster_test_images.dart' as fixtures;

RasterCacheKey key(String owner, {int generation = 1, Object pixels = 0}) =>
    RasterCacheKey(
      generation: generation,
      kind: 'tile',
      owner: owner,
      pixels: pixels,
    );

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late RasterImageStore store;
  final leases = <RasterImageLease>[];
  RasterImageLease put(
    String owner,
    int width,
    int height, {
    int generation = 1,
    Object pixels = 0,
  }) {
    final lease = store.insertAndAcquire(
      key(owner, generation: generation, pixels: pixels),
      fixtures.image(width, height),
      1,
      Size(width.toDouble(), height.toDouble()),
    );
    leases.add(lease);
    return lease;
  }

  RasterImageLease? get(String owner, {int generation = 1, Object pixels = 0}) {
    final lease = store.acquire(
      key(owner, generation: generation, pixels: pixels),
    );
    if (lease != null) leases.add(lease);
    return lease;
  }

  setUp(
    () =>
        store = RasterImageStore(maximumBytes: 64, maximumEntries: 10)
          ..activateGeneration(1),
  );
  tearDown(() {
    for (final lease in leases) {
      lease.release();
    }
    leases.clear();
    store.clear();
    expect(store.statistics.liveBytes, 0);
  });

  test('byte LRU retains useful entries regardless of their count', () {
    put('a', 2, 2).release(); // 16
    put('b', 3, 2).release(); // 24
    get('a')!.release(); // a is newest
    put('c', 3, 3).release(); // 36; 76 total, evict b
    expect(store.contains(key('a')), isTrue);
    expect(store.contains(key('b')), isFalse);
    expect(store.contains(key('c')), isTrue);
    expect(store.statistics.retainedBytes, 52);
    expect(store.statistics.evictions, 1);
  });

  test('capacity pressure chooses historical images before current images', () {
    put('old', 2, 2).release();
    store.activateGeneration(2);
    put('current', 3, 2, generation: 2).release();
    get('old')!.release(); // even a touched historical entry loses first
    put('new', 3, 3, generation: 2).release();
    expect(store.contains(key('old')), isFalse);
    expect(store.contains(key('current', generation: 2)), isTrue);
    expect(store.statistics.retainedBytes, 60);
  });

  test('three generations reactivate without capturing again', () {
    for (final generation in [1, 2, 3]) {
      store.activateGeneration(generation);
      put('a', 2, 2, generation: generation).release();
    }
    store.activateGeneration(1);
    expect(get('a')!.image.width, 2);
    expect(store.statistics.captures, 3);
    store.activateGeneration(4);
    expect(store.contains(key('a', generation: 2)), isFalse);
    expect(store.contains(key('a')), isTrue);
    expect(store.contains(key('a', generation: 3)), isTrue);
  });

  test(
    'oversized displayed image remains discoverable without a capture loop',
    () {
      final shown = put('huge', 8, 8);
      expect(store.statistics.retainedBytes, 256);
      expect(store.statistics.pinnedBytes, 256);
      expect(get('huge')!.image, same(shown.image));
      expect(store.statistics.captures, 1);
      leases.last.release();
      shown.release();
      expect(store.contains(key('huge')), isFalse);
      expect(shown.image.debugDisposed, isTrue);
    },
  );

  test('replacement preserves old displayed image until its final lease', () {
    final old = put('a', 2, 2);
    final other = get('a')!;
    final replacement = put('a', 3, 2);
    expect(store.statistics.retainedBytes, 24);
    expect(store.statistics.retiredLiveBytes, 16);
    expect(store.statistics.pinnedBytes, 24);
    expect(store.statistics.liveBytes, 40);
    old.release();
    old.release(); // idempotent
    expect(old.image.debugDisposed, isFalse);
    other.release();
    expect(old.image.debugDisposed, isTrue);
    expect(replacement.image.debugDisposed, isFalse);
    expect(store.statistics.retiredLiveBytes, 0);
  });

  test('a newer revision prunes stale unpinned pixels of the same owner', () {
    put('a', 2, 2, pixels: 'old').release();
    put('a', 3, 2, pixels: 'new').release();
    expect(store.contains(key('a', pixels: 'old')), isFalse);
    expect(store.contains(key('a', pixels: 'new')), isTrue);
    expect(store.statistics.retainedBytes, 24);
    expect(store.statistics.evictions, 0);
    expect(store.prunedRevisions, 1);
  });

  test('a live older geometry variant survives until its lease ends', () {
    final old = put('a', 2, 2);
    put('a', 3, 2, pixels: 'new').release();
    expect(store.contains(key('a')), isTrue);
    old.release();
    expect(store.contains(key('a')), isFalse);
    expect(store.statistics.retainedBytes, 24);
    expect(store.statistics.evictions, 0);
  });

  test('pressure keeps displayed images and drops them on release', () {
    final shown = put('a', 2, 2);
    put('b', 2, 2).release();
    store.trimForMemoryPressure();
    expect(store.contains(key('a')), isTrue);
    expect(store.contains(key('b')), isFalse);
    expect(store.statistics.pinnedBytes, 16);
    shown.release();
    expect(store.contains(key('a')), isFalse);
    expect(store.statistics.retiredLiveBytes, 0);
    expect(shown.image.debugDisposed, isTrue);
  });

  test('reuse before release re-admits an image marked by pressure', () {
    final shown = put('a', 2, 2);
    store.trimForMemoryPressure();
    final again = get('a')!;
    again.release();
    shown.release();
    expect(store.contains(key('a')), isTrue);
    expect(store.statistics.retainedBytes, 16);
  });

  test('repeated pressure is idempotent and leaves no live bytes', () {
    put('a', 2, 2).release();
    put('b', 2, 2).release();
    store.trimForMemoryPressure();
    store.trimForMemoryPressure();
    expect(store.statistics.retainedBytes, 0);
    expect(store.statistics.liveBytes, 0);
    expect(store.statistics.entryCount, 0);
  });

  test('clear detaches keys but keeps displayed images alive', () {
    final a = put('a', 2, 2);
    put('b', 3, 2).release();
    store.clear();
    expect(get('a'), isNull);
    expect(store.statistics.retainedBytes, 0);
    expect(store.statistics.retiredLiveBytes, 16);
    expect(a.image.debugDisposed, isFalse);
    a.release();
    expect(a.image.debugDisposed, isTrue);
    expect(store.statistics.liveBytes, 0);
  });

  test(
    'owner invalidation clears all generations without harming another owner',
    () {
      final a = put('a', 2, 2);
      put('b', 2, 2).release();
      store.activateGeneration(2);
      put('a', 2, 2, generation: 2).release();
      store.invalidateOwner('a');
      expect(store.contains(key('a')), isFalse);
      expect(store.contains(key('a', generation: 2)), isFalse);
      expect(store.contains(key('b')), isTrue);
      expect(a.image.debugDisposed, isFalse);
      store.retainOwners({});
      expect(store.statistics.retainedBytes, 0);
    },
  );

  test(
    'speculation never evicts the current working set or duplicates a key',
    () {
      put('a', 4, 3).release(); // 48
      expect(store.canWarm(key: key('a'), estimatedBytes: 4), isFalse);
      expect(store.canWarm(key: key('b'), estimatedBytes: 16), isTrue);
      expect(store.canWarm(key: key('b'), estimatedBytes: 20), isFalse);
      store.activateGeneration(2);
      expect(
        store.canWarm(key: key('b', generation: 2), estimatedBytes: 64),
        isTrue,
      );
      expect(
        store.canWarm(key: key('b', generation: 1), estimatedBytes: 4),
        isFalse,
      );
    },
  );

  test(
    'peak includes transient insertion and stats reset preserves live bytes',
    () {
      put('a', 4, 3).release();
      put('b', 4, 3).release();
      expect(store.statistics.peakLiveBytes, 96);
      store.resetStatistics();
      expect(store.statistics.retainedBytes, 48);
      expect(store.statistics.peakLiveBytes, 48);
      expect(store.statistics.captures, 0);
    },
  );

  test('entry cap is secondary and never evicts a displayed key', () {
    store = RasterImageStore(maximumBytes: 1024, maximumEntries: 1)
      ..activateGeneration(1);
    final pinned = put('a', 1, 1);
    put('b', 1, 1).release();
    expect(store.contains(key('a')), isTrue);
    expect(store.contains(key('b')), isFalse);
    expect(store.statistics.retainedBytes, 4);
    pinned.release();
  });
}
