// flutter drive --profile --no-dds -d DEVICE \
//   --driver=test_driver/performance.dart \
//   --target=integration_test/group_transition_performance_test.dart
import 'dart:convert';
import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:qlaunch/main.dart';
import 'package:qlaunch/src/rust/api/api.dart' show ModelSnapshot;
import 'package:qlaunch/src/rust/frb_generated.dart';
import 'package:qlaunch/src/rust/model/icon.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/state/launcher_backend.dart';
import 'package:qlaunch/src/ui/home_screen.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

/// Count real bridge work, including cache misses, without mocking the backend.
class _MeasuredBackend extends RustBackend {
  int icons = 0;
  int snapshots = 0;
  int iconMicros = 0;

  @override
  Future<IconImage?> iconFor(
    String package,
    String activity,
    int sizePx,
  ) async {
    icons++;
    final watch = Stopwatch()..start();
    final task = TimelineTask()
      ..start('bridge.iconFor', arguments: {'package': package});
    try {
      return await super.iconFor(package, activity, sizePx);
    } finally {
      iconMicros += watch.elapsedMicroseconds;
      task.finish();
    }
  }

  @override
  Future<ModelSnapshot> snapshot() async {
    snapshots++;
    final task = TimelineTask()..start('bridge.snapshot');
    try {
      return await super.snapshot();
    } finally {
      task.finish();
    }
  }

  void reset() {
    icons = snapshots = iconMicros = 0;
  }
}

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized()
    ..framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;

  testWidgets('Games ↔ Apps: renderer and bridge visibility matrix', (
    tester,
  ) async {
    PaintingBinding.instance.imageCache
      ..maximumSize = 1000
      ..maximumSizeBytes = 64 << 20;
    await SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    await RustLib.init();
    final backend = _MeasuredBackend();
    await tester.pumpWidget(QLaunchApp(backendOverride: backend));
    await tester.pumpAndSettle();
    final state = tester.widget<HomeScreen>(find.byType(HomeScreen)).state;
    final original = state.snapshot!.settings;
    addTearDown(
      () => state.updateSettings(
        SettingsPatch(
          continuousScroll: original.continuousScroll,
          iconsVisible: original.iconsVisible,
          labelsVisible: original.labelsVisible,
          labelShadow: original.labelShadow,
          scrollFade: original.scrollFade,
        ),
      ),
    );
    final apps = state.groups.firstWhere((g) => g.role == GroupRole.apps);
    final games = state.groups.firstWhere((g) => g.role == GroupRole.games);
    debugPrint(
      'TRANSITION_CONFIG ${jsonEncode({'continuous': original.continuousScroll, 'icons': original.iconsVisible, 'labels': original.labelsVisible, 'fade': original.scrollFade.name, 'shadow': original.labelShadow, 'apps': state.itemsIn(apps.id).length, 'games': state.itemsIn(games.id).length, 'columns': original.gridColumnsPortrait})}',
    );

    Future<void> go(String id, String label) async {
      final tabs = tester.widget<TabStrip>(find.byType(TabStrip));
      if (tabs.currentGroupId == id) return;
      final title = find
          .descendant(of: find.byType(TabStrip), matching: find.text(label))
          .hitTestable()
          .first;
      await tester.tap(title);
      await tester.pumpAndSettle(const Duration(milliseconds: 16));
      expect(tester.widget<TabStrip>(find.byType(TabStrip)).currentGroupId, id);
    }

    Future<void> backAndForth() async {
      for (var i = 0; i < 10; i++) {
        final task = TimelineTask()..start('transition.appsToGames');
        await go(games.id, games.label);
        task.finish();
        final back = TimelineTask()..start('transition.gamesToApps');
        await go(apps.id, apps.label);
        back.finish();
      }
    }

    if (!const bool.fromEnvironment('DIAGNOSTICS_ONLY')) {
      for (final continuous in [
        original.continuousScroll,
        !original.continuousScroll,
      ]) {
        for (final visibility in [
          (true, true),
          (false, true),
          (true, false),
          (false, false),
        ]) {
          await state.updateSettings(
            SettingsPatch(
              continuousScroll: continuous,
              iconsVisible: visibility.$1,
              labelsVisible: visibility.$2,
              labelShadow: true,
            ),
          );
          await tester.pumpAndSettle();
          await go(apps.id, apps.label);
          await go(games.id, games.label);
          await go(apps.id, apps.label);
          backend.reset();
          final key =
              '${continuous ? 'continuous' : 'paged'}_icons${visibility.$1}_text${visibility.$2}';
          await binding.watchPerformance(backAndForth, reportKey: key);
          final data = binding.reportData![key] as Map;
          debugPrint(
            'TRANSITION_RESULT ${jsonEncode({'case': key, 'frames': data['frame_count'], 'buildP99': data['99th_percentile_frame_build_time_millis'], 'buildWorst': data['worst_frame_build_time_millis'], 'rasterP99': data['99th_percentile_frame_rasterizer_time_millis'], 'rasterWorst': data['worst_frame_rasterizer_time_millis'], 'rasterMisses': data['missed_frame_rasterizer_budget_count'], 'iconCalls': backend.icons, 'snapshotCalls': backend.snapshots, 'iconCallTotalMs': backend.iconMicros / 1000})}',
          );
        }
      }
    }

    for (final variant in [
      ('paged_default', true, original.scrollFade),
      ('paged_no_shadow', false, original.scrollFade),
      ('paged_no_fade', true, ScrollFadeStyle.none),
      ('paged_no_shadow_no_fade', false, ScrollFadeStyle.none),
    ]) {
      await state.updateSettings(
        SettingsPatch(
          continuousScroll: false,
          iconsVisible: true,
          labelsVisible: true,
          labelShadow: variant.$2,
          scrollFade: variant.$3,
        ),
      );
      await tester.pumpAndSettle();
      await go(apps.id, apps.label);
      await go(games.id, games.label);
      await go(apps.id, apps.label);
      backend.reset();
      await binding.watchPerformance(backAndForth, reportKey: variant.$1);
      final data = binding.reportData![variant.$1] as Map;
      debugPrint(
        'TRANSITION_DIAGNOSTIC ${jsonEncode({'case': variant.$1, 'frames': data['frame_count'], 'buildP99': data['99th_percentile_frame_build_time_millis'], 'rasterP99': data['99th_percentile_frame_rasterizer_time_millis'], 'rasterWorst': data['worst_frame_rasterizer_time_millis'], 'rasterMisses': data['missed_frame_rasterizer_budget_count'], 'iconCalls': backend.icons, 'snapshotCalls': backend.snapshots})}',
      );
    }
    if (const bool.fromEnvironment('TRACE_TRANSITIONS')) {
      await state.updateSettings(
        SettingsPatch(
          continuousScroll: false,
          iconsVisible: true,
          labelsVisible: true,
          labelShadow: original.labelShadow,
          scrollFade: original.scrollFade,
        ),
      );
      await tester.pumpAndSettle();
      await go(apps.id, apps.label);
      await binding.traceAction(
        backAndForth,
        streams: ['Dart', 'Embedder', 'GC'],
        reportKey: 'paged_trace',
      );
    }

    // Exercise real cache misses too; warm transitions alone cannot rule out a
    // separate first-use icon problem. Clearing source/decoded caches while
    // icons are hidden approximates an app restart with the persisted disk cache.
    await state.updateSettings(
      SettingsPatch(
        continuousScroll: false,
        iconsVisible: false,
        labelsVisible: false,
        labelShadow: true,
        scrollFade: original.scrollFade,
      ),
    );
    await tester.pumpAndSettle();
    state.releaseIconSources();
    PaintingBinding.instance.imageCache
      ..clear()
      ..clearLiveImages();
    backend.reset();
    await binding.watchPerformance(() async {
      await state.updateSettings(const SettingsPatch(iconsVisible: true));
      await tester.pumpAndSettle();
      await backAndForth();
    }, reportKey: 'cold_icons_only');
    final cold = binding.reportData!['cold_icons_only'] as Map;
    debugPrint(
      'TRANSITION_COLD ${jsonEncode({'buildP99': cold['99th_percentile_frame_build_time_millis'], 'buildWorst': cold['worst_frame_build_time_millis'], 'rasterP99': cold['99th_percentile_frame_rasterizer_time_millis'], 'rasterWorst': cold['worst_frame_rasterizer_time_millis'], 'rasterMisses': cold['missed_frame_rasterizer_budget_count'], 'iconCalls': backend.icons, 'snapshotCalls': backend.snapshots, 'iconCallTotalMs': backend.iconMicros / 1000})}',
    );

    await state.updateSettings(const SettingsPatch(labelsVisible: true));
    await tester.pumpAndSettle();
    await go(apps.id, apps.label);
    backend.reset();
    await binding.watchPerformance(() async {
      for (var i = 0; i < 10; i++) {
        await tester.fling(find.byType(PageView), const Offset(250, 0), 800);
        await tester.pumpAndSettle(const Duration(milliseconds: 16));
        expect(
          tester.widget<TabStrip>(find.byType(TabStrip)).currentGroupId,
          games.id,
        );
        await tester.fling(find.byType(PageView), const Offset(-250, 0), 800);
        await tester.pumpAndSettle(const Duration(milliseconds: 16));
        expect(
          tester.widget<TabStrip>(find.byType(TabStrip)).currentGroupId,
          apps.id,
        );
      }
    }, reportKey: 'paged_swipes');
    final swipes = binding.reportData!['paged_swipes'] as Map;
    debugPrint(
      'TRANSITION_SWIPES ${jsonEncode({'buildP99': swipes['99th_percentile_frame_build_time_millis'], 'buildWorst': swipes['worst_frame_build_time_millis'], 'rasterP99': swipes['99th_percentile_frame_rasterizer_time_millis'], 'rasterWorst': swipes['worst_frame_rasterizer_time_millis'], 'rasterMisses': swipes['missed_frame_rasterizer_budget_count'], 'iconCalls': backend.icons, 'snapshotCalls': backend.snapshots})}',
    );
    const maxRasterP99 = int.fromEnvironment(
      'MAX_TRANSITION_RASTER_P99_MS',
      defaultValue: 25,
    );
    final baseline = binding.reportData!['paged_default'] as Map;
    expect(
      baseline['99th_percentile_frame_rasterizer_time_millis'],
      lessThan(maxRasterP99),
      reason:
          'Games ↔ Apps with labels and shadows must not stall the renderer.',
    );
  });
}
