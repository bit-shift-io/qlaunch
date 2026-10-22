import 'dart:convert';
import 'dart:developer';
import 'dart:math' as math;
import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/raster_cache_stats.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';

class RasterScenarioResult {
  RasterScenarioResult({
    required this.name,
    required this.refreshRateHz,
    required this.timings,
    required this.before,
    required this.after,
    required this.sourceBytes,
    required this.decodedBytes,
    required this.elapsedMicros,
    this.extraStats = const <String, Object>{},
  });

  final String name;
  final double refreshRateHz;
  final List<FrameTiming> timings;
  final RasterCacheStats before, after;
  final int sourceBytes, decodedBytes, elapsedMicros;
  final Map<String, Object> extraStats;

  Map<String, Object> toJson() {
    Map<String, Object> summarize(List<int> values) {
      values.sort();
      double percentile(double p) => values.isEmpty
          ? 0
          : values[math.max(0, (values.length * p).ceil() - 1)] / 1000;
      return {
        'p50': percentile(.5),
        'p90': percentile(.9),
        'p99': percentile(.99),
        'worst': percentile(1),
        'overRefreshBudget': values
            .where((v) => v > 1e6 / refreshRateHz)
            .length,
        'over16ms': values.where((v) => v > 16000).length,
      };
    }

    return {
      'name': name,
      'refreshRateHz': refreshRateHz,
      'frames': timings.length,
      'elapsedMs': elapsedMicros / 1000,
      'build': summarize([
        for (final t in timings) t.buildDuration.inMicroseconds,
      ]),
      'raster': summarize([
        for (final t in timings) t.rasterDuration.inMicroseconds,
      ]),
      'before': before.toJson(),
      'after': after.toJson(),
      'newCaptures': after.captures - before.captures,
      'newEvictions': after.evictions - before.evictions,
      'scheduler': extraStats,
      'sourceBytes': sourceBytes,
      'decodedBytes': decodedBytes,
    };
  }
}

/// Use exactly the same action window for percentiles and refresh-budget counts.
/// FrameTiming callbacks arrive in batches after rendering; flush and filter by
/// their engine timestamps rather than diluting results with idle frames.
Future<RasterScenarioResult> measureRasterScenario({
  required String name,
  required WidgetTester tester,
  required Future<void> Function() action,
  required int Function() sourceBytes,
  Map<String, Object> Function()? extraStats,
  Future<void> Function()? settle,
}) async {
  final hz = tester.view.display.refreshRate;
  if (!hz.isFinite || hz <= 0)
    throw StateError('Invalid display refresh rate: $hz');
  await Future<void>.delayed(const Duration(milliseconds: 150));
  final timings = <FrameTiming>[];
  void collect(List<FrameTiming> batch) => timings.addAll(batch);
  tester.binding.addTimingsCallback(collect);
  final before = TileRasterCache.statistics;
  final start = Timeline.now;
  late int end;
  final task = TimelineTask()..start('raster.$name');
  try {
    await action();
    if (settle != null) await settle();
    end = Timeline.now;
    await Future<void>.delayed(const Duration(milliseconds: 150));
  } finally {
    task.finish();
    tester.binding.removeTimingsCallback(collect);
  }
  final result = RasterScenarioResult(
    name: name,
    refreshRateHz: hz,
    timings: timings.where((t) {
      final built = t.timestampInMicroseconds(FramePhase.buildStart);
      return built >= start && built <= end;
    }).toList(),
    before: before,
    after: TileRasterCache.statistics,
    sourceBytes: sourceBytes(),
    decodedBytes: PaintingBinding.instance.imageCache.currentSizeBytes,
    elapsedMicros: end - start,
    extraStats: extraStats?.call() ?? const <String, Object>{},
  );
  // The Android logger truncates long lines, so emit base64 chunks that the
  // analysis side reassembles. `binding.reportData` carries the same objects.
  final encoded = base64Encode(utf8.encode(jsonEncode(result.toJson())));
  const chunk = 600;
  final total = (encoded.length / chunk).ceil();
  for (var i = 0; i < total; i++) {
    final start = i * chunk;
    final end = math.min(start + chunk, encoded.length);
    debugPrintSynchronously(
      'RASTER_JSON $i/$total ${encoded.substring(start, end)}',
    );
  }
  return result;
}
