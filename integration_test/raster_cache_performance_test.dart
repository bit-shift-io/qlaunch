// See docs/scroll-performance.md for profile-mode commands and output schema.
import 'dart:convert';
import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:qlaunch/main.dart';
import 'package:qlaunch/src/rust/frb_generated.dart';
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/ui/app_grid.dart';
import 'package:qlaunch/src/ui/home_screen.dart';
import 'package:qlaunch/src/ui/raster_capture_scheduler.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

import 'support/raster_performance.dart';

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized()
    ..framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  testWidgets('all-tab raster working set and paging performance', (
    tester,
  ) async {
    PaintingBinding.instance.imageCache
      ..maximumSize = 1000
      ..maximumSizeBytes = 64 << 20;
    await SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    await RustLib.init();
    await tester.pumpWidget(const QLaunchApp());
    await tester.pumpAndSettle();
    final state = tester.widget<HomeScreen>(find.byType(HomeScreen)).state;
    final original = state.snapshot!.settings;
    addTearDown(() async {
      TileRasterCache.mode = TileRasterMode.none;
      await state.updateSettings(
        SettingsPatch(
          continuousScroll: original.continuousScroll,
          iconsVisible: original.iconsVisible,
          labelsVisible: original.labelsVisible,
          labelShadow: original.labelShadow,
          labelShadowBlur: original.labelShadowBlur,
          tabVisibility: original.tabVisibility,
        ),
      );
      await tester.pumpAndSettle();
      TileRasterCache.reset();
    });
    final ids = state.pageIds.toList();
    expect(ids, isNotEmpty);
    final media = MediaQuery.of(tester.element(find.byType(HomeScreen)));
    debugPrint(
      'RASTER_CONFIG ${jsonEncode({
        'viewport': [media.size.width, media.size.height],
        'dpr': media.devicePixelRatio,
        'refreshRateHz': tester.view.display.refreshRate,
        'original': {'blur': original.labelShadowBlur, 'continuous': original.continuousScroll, 'icons': original.iconsVisible, 'labels': original.labelsVisible},
        'groups': [
          for (final id in ids) {'id': id, 'label': state.groups.firstWhere((g) => g.id == id).label, 'items': state.itemsIn(id).length},
        ],
      })}',
    );
    final visited = <String>{};
    final scrolled = <String>{};
    Future<void> nextTab() async {
      final current = tester
          .widget<TabStrip>(find.byType(TabStrip))
          .currentGroupId;
      final target = ids[(ids.indexOf(current) + 1) % ids.length];
      await tester.tap(find.byKey(TabStrip.tapKey(1)).hitTestable().first);
      await tester.pumpAndSettle(const Duration(milliseconds: 16));
      final actual = tester
          .widget<TabStrip>(find.byType(TabStrip))
          .currentGroupId;
      expect(actual, target);
      visited.add(actual);
    }

    Future<void> scrollGroup() async {
      final id = tester.widget<TabStrip>(find.byType(TabStrip)).currentGroupId;
      if (state.itemsIn(id).isEmpty) return;
      final grid = find
          .descendant(
            of: find.byWidgetPredicate((w) => w is AppGrid && w.groupId == id),
            matching: find.byType(GridView),
          )
          .hitTestable()
          .first;
      final scroll = tester.state<ScrollableState>(
        find.descendant(of: grid, matching: find.byType(Scrollable)).first,
      );
      for (
        var step = 0;
        scroll.position.pixels < scroll.position.maxScrollExtent - 1;
        step++
      ) {
        expect(step, lessThan(100), reason: 'must reach bottom of $id');
        await tester.fling(grid, const Offset(0, -260), 1600);
        await tester.pumpAndSettle(const Duration(milliseconds: 16));
      }
      expect(scroll.position.extentAfter, lessThanOrEqualTo(1));
      scrolled.add(id);
      // A deterministic jump back does not generate synthetic overscroll time.
      scroll.position.jumpTo(0);
      await tester.pumpAndSettle(const Duration(milliseconds: 16));
      expect(scroll.position.pixels, 0);
    }

    Future<void> circuit({bool scroll = false}) async {
      for (var i = 0; i < ids.length; i++) {
        await nextTab();
        if (scroll) await scrollGroup();
      }
    }

    Future<void> swipes({bool reverse = false}) async {
      for (var i = 0; i < 12; i++) {
        await tester.fling(find.byType(PageView), const Offset(-420, 0), 2400);
        if (reverse) {
          await tester.pump(const Duration(milliseconds: 32));
          await tester.fling(find.byType(PageView), const Offset(420, 0), 2400);
        }
        await tester.pumpAndSettle(const Duration(milliseconds: 16));
        final tab = tester.widget<TabStrip>(find.byType(TabStrip));
        final page = tester
            .widget<PageView>(find.byType(PageView))
            .controller!
            .page!;
        expect(tab.controller!.groupIdAt(page.round()), tab.currentGroupId);
      }
    }

    const schedulerFlag = String.fromEnvironment(
      'RASTER_SCHEDULER',
      defaultValue: 'immediate',
    );
    void applySchedulerVariant() {
      TileRasterCache.scheduler.dispose();
      TileRasterCache.immediateVisibleCapture = schedulerFlag == 'immediate';
      final perFrame = schedulerFlag == 'unbounded'
          ? 1 << 30
          : int.tryParse(schedulerFlag) ?? 4;
      TileRasterCache.scheduler = RasterCaptureScheduler(
        maximumVisiblePerFrame: perFrame,
      );
      debugPrint(
        'RASTER_SCHEDULER_CONFIG ${jsonEncode({'variant': schedulerFlag, 'immediate': TileRasterCache.immediateVisibleCapture, 'maximumVisiblePerFrame': perFrame})}',
      );
    }

    /// Waits until every visible capture the scheduler queued has finished, so
    /// moving work past the action cannot hide it from the measurement.
    Future<void> drainCaptures() async {
      for (var attempt = 0; attempt < 400; attempt++) {
        final stats = TileRasterCache.scheduler.statistics();
        if (stats['visibleQueued'] == 0 && stats['inFlightVisible'] == 0) {
          return;
        }
        await tester.pump(const Duration(milliseconds: 16));
      }
    }

    final results = <Map<String, Object>>[];
    Future<void> measure(String name, Future<void> Function() action) async {
      final result = await measureRasterScenario(
        name: name,
        tester: tester,
        action: action,
        sourceBytes: () => state.iconCacheBytes,
        extraStats: () => {
          'variant': schedulerFlag,
          'immediate': TileRasterCache.immediateVisibleCapture,
          'maximumVisiblePerFrame':
              TileRasterCache.scheduler.maximumVisiblePerFrame,
          ...TileRasterCache.scheduler.statistics(),
        },
        settle: drainCaptures,
      );
      results.add(result.toJson());
    }

    const samples = int.fromEnvironment('RASTER_SAMPLES', defaultValue: 3);
    const modesFlag = String.fromEnvironment(
      'RASTER_MODES',
      defaultValue: 'all',
    );
    final modes = modesFlag == 'all'
        ? [
            TileRasterMode.none,
            TileRasterMode.separate,
            TileRasterMode.combined,
          ]
        : [TileRasterMode.values.byName(modesFlag)];
    for (var sample = 0; sample < samples; sample++) {
      final order = sample.isOdd ? modes.reversed : modes;
      for (final mode in order) {
        TileRasterCache.mode = TileRasterMode.none;
        await state.updateSettings(
          const SettingsPatch(
            continuousScroll: false,
            iconsVisible: true,
            labelsVisible: true,
            labelShadow: false,
            labelShadowBlur: 5,
            tabVisibility: TabVisibility.always,
          ),
        );
        await tester.pumpAndSettle();
        TileRasterCache.reset();
        applySchedulerVariant();
        TileRasterCache.mode = mode;
        final prefix = '${mode.name}_$sample';
        await measure('${prefix}_settings', () async {
          await state.updateSettings(const SettingsPatch(labelShadow: true));
          await tester.pumpAndSettle();
        });
        await measure('${prefix}_cold_circuit', () => circuit(scroll: true));
        await measure('${prefix}_repeat_circuit', () => circuit(scroll: true));
        await measure('${prefix}_warm_paging', () async {
          for (var i = 0; i < 3; i++) {
            await circuit();
          }
        });
        if (mode != TileRasterMode.none) {
          // Pressure recovery is its own scenario, not part of the
          // zero-recapture warm-navigation contract.
          await measure('${prefix}_pressure_trim', () async {
            // Land on a populated group so visible cells exercise the trim.
            for (var tries = 0; tries < ids.length; tries++) {
              final current = tester
                  .widget<TabStrip>(find.byType(TabStrip))
                  .currentGroupId;
              if (state.itemsIn(current).isNotEmpty) break;
              await nextTab();
            }
            tester.binding.handleMemoryPressure();
            await tester.pumpAndSettle();
            expect(find.byType(TabStrip), findsOneWidget);
            expect(tester.takeException(), isNull);
          });
          await measure('${prefix}_pressure_revisit', () => circuit());
        }
        await measure('${prefix}_fast_swipes', () => swipes());
        await measure('${prefix}_reversals', () => swipes(reverse: true));
        if (mode != TileRasterMode.none) {
          for (final blur in [6, 7, 5]) {
            await measure('${prefix}_blur${blur}_settings', () async {
              await state.updateSettings(SettingsPatch(labelShadowBlur: blur));
              await tester.pumpAndSettle();
            });
            await measure(
              '${prefix}_blur${blur}_circuit',
              () => circuit(scroll: true),
            );
          }
        }
      }
    }
    expect(visited, containsAll(ids));
    expect(
      scrolled,
      containsAll(ids.where((id) => state.itemsIn(id).isNotEmpty)),
    );
    binding.reportData = {'rasterResults': results};
    if (const bool.fromEnvironment('RASTER_TRACE')) {
      await binding.traceAction(
        () => swipes(reverse: true),
        streams: ['Dart', 'Embedder', 'GC'],
        reportKey: 'warm_swipe_trace',
      );
    }
    Timeline.instantSync('raster.complete');
  });
}
