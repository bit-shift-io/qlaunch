// Run on a device with:
// flutter drive --profile --no-dds -d DEVICE --driver=test_driver/performance.dart \
//   --target=integration_test/scroll_performance_test.dart
// Uses the installed launcher's real groups and icons, without replacing data.
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:qlaunch/main.dart' as app;
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/ui/app_cell.dart';
import 'package:qlaunch/src/ui/home_screen.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  // Use actual engine-driven animation frames, rather than the test binding's
  // default policy that only renders requested test/pointer-fade frames.
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;

  testWidgets('real-device continuous scrolling performance', (tester) async {
    await app.main();
    await tester.pumpAndSettle();
    final state = tester.widget<HomeScreen>(find.byType(HomeScreen)).state;
    final originalMode = state.snapshot!.settings.continuousScroll;
    final originalFade = state.snapshot!.settings.scrollFade;
    if (!originalMode) {
      await state.updateSettings(const SettingsPatch(continuousScroll: true));
      await tester.pumpAndSettle();
    }
    addTearDown(() async {
      await state.updateSettings(
        SettingsPatch(continuousScroll: originalMode, scrollFade: originalFade),
      );
    });
    final scroller = find.byKey(const ValueKey('groupScroller'));
    expect(scroller, findsOneWidget);
    debugPrint(
      'SCROLL_STRUCTURE ${jsonEncode({'items': state.snapshot!.items.length, 'groups': state.pageIds.length, 'mountedCells': find.byType(AppCell).evaluate().length, 'scrollables': find.byType(Scrollable).evaluate().length})}',
    );

    Future<void> scrollPass() async {
      for (var i = 0; i < 12; i++) {
        await tester.fling(scroller, Offset(0, i < 6 ? -600 : 600), 1800);
        await tester.pumpAndSettle();
      }
    }

    await binding.watchPerformance(scrollPass, reportKey: 'scroll_first');
    await binding.watchPerformance(scrollPass, reportKey: 'scroll_warm');
    await binding.watchPerformance(() async {
      for (var i = 0; i < 12; i++) {
        await tester.tap(find.byKey(TabStrip.tabKey(1)));
        await tester.pumpAndSettle();
      }
    }, reportKey: 'section_navigation');
    await state.updateSettings(
      const SettingsPatch(scrollFade: ScrollFadeStyle.none),
    );
    await tester.pumpAndSettle();
    await binding.watchPerformance(scrollPass, reportKey: 'scroll_no_fade');
    for (final entry in binding.reportData!.entries) {
      final data = Map<String, dynamic>.from(entry.value as Map);
      data.removeWhere(
        (key, _) => !{
          'average_frame_build_time_millis',
          '99th_percentile_frame_build_time_millis',
          'average_frame_rasterizer_time_millis',
          '99th_percentile_frame_rasterizer_time_millis',
          'missed_frame_build_budget_count',
          'missed_frame_rasterizer_budget_count',
          'frame_count',
        }.contains(key),
      );
      debugPrint('SCROLL_PERFORMANCE ${entry.key} ${jsonEncode(data)}');
    }
    // A 60 Hz frame has 16.7 ms; allow 1.5 frames for the slowest 1%.
    // Override for devices with a different target or an intentional stress run.
    const maxRasterP99 = int.fromEnvironment(
      'MAX_SCROLL_RASTER_P99_MS',
      defaultValue: 25,
    );
    final warm = binding.reportData!['scroll_warm'] as Map;
    expect(
      warm['99th_percentile_frame_rasterizer_time_millis'],
      lessThan(maxRasterP99),
      reason: 'Warm scrolling must not repeatedly stall rendering.',
    );
  });
}
