import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';

import 'raster_test_images.dart' as fixtures;

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  tearDown(TileRasterCache.reset);
  const key = RasterCacheKey(
    generation: 0,
    kind: 'tile',
    owner: 'a',
    pixels: 0,
  );

  test('resetting counters preserves retained image payload and its peak', () {
    final lease = TileRasterCache.store.insertAndAcquire(
      key,
      fixtures.image(3, 2),
      1,
      const Size(3, 2),
    );
    lease.release();
    expect(TileRasterCache.statistics.retainedBytes, 24);
    expect(TileRasterCache.statistics.captures, 1);
    TileRasterCache.resetStatistics();
    expect(TileRasterCache.statistics.retainedBytes, 24);
    expect(TileRasterCache.statistics.peakLiveBytes, 24);
    expect(TileRasterCache.statistics.captures, 0);
  });

  test('cleared displayed image stays accounted until its last lease', () {
    final lease = TileRasterCache.store.insertAndAcquire(
      key,
      fixtures.image(3, 2),
      1,
      const Size(3, 2),
    );
    TileRasterCache.reset();
    expect(TileRasterCache.statistics.retainedBytes, 0);
    expect(TileRasterCache.statistics.retiredLiveBytes, 24);
    expect(TileRasterCache.statistics.liveBytes, 24);
    lease.release();
    expect(TileRasterCache.statistics.liveBytes, 0);
    expect(lease.image.debugDisposed, isTrue);
  });
}
