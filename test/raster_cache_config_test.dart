import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/raster_image_store.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

import 'fake_backend.dart';
import 'raster_test_images.dart' as fixtures;
import 'snapshot_builders.dart';

void main() {
  setUp(() => TileRasterCache.mode = TileRasterMode.automatic);
  tearDown(TileRasterCache.reset);

  test('automatic follows icon and label visibility', () {
    expect(
      TileRasterCache.modeFor(iconsVisible: true, labelsVisible: true),
      TileRasterMode.combined,
    );
    expect(
      TileRasterCache.modeFor(iconsVisible: false, labelsVisible: true),
      TileRasterMode.separate,
    );
    expect(
      TileRasterCache.modeFor(iconsVisible: true, labelsVisible: false),
      TileRasterMode.none,
    );
  });

  test('an explicit mode ignores visibility', () {
    TileRasterCache.mode = TileRasterMode.combined;
    expect(
      TileRasterCache.modeFor(iconsVisible: false, labelsVisible: false),
      TileRasterMode.combined,
    );
    TileRasterCache.mode = TileRasterMode.none;
    expect(
      TileRasterCache.modeFor(iconsVisible: true, labelsVisible: true),
      TileRasterMode.none,
    );
    expect(TileRasterCache.enabled, isFalse);
  });

  test('configure applies limits and starts from detached store', () {
    final before = TileRasterCache.store;
    before
        .insertAndAcquire(
          const RasterCacheKey(
            generation: 0,
            kind: 'tile',
            owner: 'item:a',
            pixels: 1,
          ),
          fixtures.image(4, 4),
          1,
          const Size(4, 4),
        )
        .release();
    TileRasterCache.configure(
      const RasterCacheConfig(
        maximumBytes: 16,
        maximumEntries: 2,
        recentGenerations: 1,
        mode: TileRasterMode.combined,
      ),
    );
    expect(TileRasterCache.store, isNot(same(before)));
    // The previous store releases its discoverable pixels rather than leaking.
    expect(before.statistics.retainedBytes, 0);
    expect(before.statistics.liveBytes, 0);
    expect(TileRasterCache.mode, TileRasterMode.combined);
    expect(TileRasterCache.store.maximumBytes, 16);
    expect(TileRasterCache.store.statistics.retainedBytes, 0);
  });

  testWidgets('automatic still caches tabs and headings with icons only', (
    tester,
  ) async {
    final store = RasterImageStore()..activateGeneration(0);
    addTearDown(store.clear);
    final controller = GroupPagerController(pageIds: const ['a', 'b']);
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: RasterCacheScope(
          store: store,
          generation: 0,
          child: Scaffold(
            body: TabStrip(
              pageIds: controller.pageIds,
              groups: [for (final id in controller.pageIds) makeGroup(id)],
              currentGroupId: 'a',
              settings: settingsWith(iconsVisible: false),
              controller: controller,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(RawImage), findsWidgets);
  });
}
