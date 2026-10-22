import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';

import 'raster_test_images.dart' as fixtures;

class _RejectingStore extends RasterImageStore {
  bool reject = true;

  @override
  RasterImageLease insertAndAcquire(
    RasterCacheKey key,
    ui.Image image,
    double pixelRatio,
    ui.Size logicalSize,
  ) {
    if (reject) {
      image.dispose();
      throw StateError('capture rejected');
    }
    return super.insertAndAcquire(key, image, pixelRatio, logicalSize);
  }
}

void main() {
  late RasterImageStore store;
  setUp(() {
    store = RasterImageStore()..activateGeneration(0);
    TileRasterCache.mode = TileRasterMode.combined;
  });
  tearDown(() {
    store.clear();
    TileRasterCache.reset();
  });
  const key = RasterCacheKey(
    generation: 0,
    kind: 'legacy',
    owner: 'legacy',
    pixels: 'tile',
  );
  Future<void> pump(
    WidgetTester tester, {
    bool ready = false,
    String signature = 'tile',
  }) => tester.pumpWidget(
    MaterialApp(
      home: Center(
        child: SizedBox(
          width: 20,
          height: 20,
          child: RasterTile(
            store: store,
            cacheKey: RasterCacheKey(
              generation: 0,
              kind: 'legacy',
              owner: 'legacy',
              pixels: signature,
            ),
            captureReady: ready,
            label: 'Maps',
            child: const ColoredBox(key: ValueKey('live'), color: Colors.red),
          ),
        ),
      ),
    ),
  );

  test('the facade trim delegates to the session store', () {
    final session = TileRasterCache.store;
    session.clear();
    final inserted = session.insertAndAcquire(
      key,
      fixtures.image(4, 4),
      1,
      const Size(4, 4),
    );
    final unpinned = session.insertAndAcquire(
      const RasterCacheKey(
        generation: 0,
        kind: 'legacy',
        owner: 'other',
        pixels: 'other',
      ),
      fixtures.image(4, 4),
      1,
      const Size(4, 4),
    );
    unpinned.release();
    TileRasterCache.trimForMemoryPressure();
    expect(session.contains(key), isTrue);
    expect(session.contains(unpinned.key), isFalse);
    inserted.release();
    expect(session.contains(key), isFalse);
  });

  testWidgets('completed hit displays on first build before icon readiness', (
    tester,
  ) async {
    store
        .insertAndAcquire(key, fixtures.image(20, 20), 1, const Size(20, 20))
        .release();
    await pump(tester);
    expect(find.byType(RawImage), findsOneWidget);
    expect(find.byKey(const ValueKey('live')), findsNothing);
    expect(store.statistics.captures, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    await pump(tester);
    expect(find.byType(RawImage), findsOneWidget);
    expect(store.statistics.captures, 1);
  });

  testWidgets('miss waits for readiness and never freezes an empty frame', (
    tester,
  ) async {
    await pump(tester);
    expect(find.byKey(const ValueKey('live')), findsOneWidget);
    expect(store.statistics.captures, 0);
    await pump(tester, ready: true);
    await tester.pump();
    expect(find.byType(RawImage), findsOneWidget);
    expect(store.statistics.captures, 1);
    store.trimForMemoryPressure();
    await pump(tester, ready: true);
    expect(find.byType(RawImage), findsOneWidget);
    expect(store.statistics.captures, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(store.statistics.liveBytes, 0);
  });

  testWidgets('a signature change never keeps showing the old capture', (
    tester,
  ) async {
    store
        .insertAndAcquire(key, fixtures.image(20, 20), 1, const Size(20, 20))
        .release();
    await pump(tester);
    expect(find.byType(RawImage), findsOneWidget);

    await pump(tester, signature: 'tile2');
    expect(find.byKey(const ValueKey('live')), findsOneWidget);
    expect(find.byType(RawImage), findsNothing);
    expect(store.statistics.captures, 1);

    await pump(tester, signature: 'tile2', ready: true);
    await tester.pump();
    expect(find.byType(RawImage), findsOneWidget);
    expect(store.statistics.captures, 2);
    expect(store.contains(key), isFalse);
  });

  testWidgets('a failed capture is reported and retried only on a new build', (
    tester,
  ) async {
    store.clear();
    final rejecting = _RejectingStore()..activateGeneration(0);
    await tester.pumpWidget(
      MaterialApp(
        home: Center(
          child: SizedBox(
            width: 20,
            height: 20,
            child: RasterTile(
              store: rejecting,
              cacheKey: key,
              child: const ColoredBox(key: ValueKey('live'), color: Colors.red),
            ),
          ),
        ),
      ),
    );
    expect(rejecting.statistics.captureFailures, 1);
    expect(rejecting.statistics.captures, 0);
    expect(find.byKey(const ValueKey('live')), findsOneWidget);

    await tester.pump();
    expect(rejecting.statistics.captureFailures, 1);
    expect(rejecting.statistics.captures, 0);

    rejecting.reject = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Center(
          child: SizedBox(
            width: 20,
            height: 20,
            child: RasterTile(
              store: rejecting,
              cacheKey: key,
              child: const ColoredBox(key: ValueKey('live'), color: Colors.red),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(rejecting.statistics.captureFailures, 1);
    expect(rejecting.statistics.captures, 1);
    expect(find.byType(RawImage), findsOneWidget);
    rejecting.clear();
  });
}
