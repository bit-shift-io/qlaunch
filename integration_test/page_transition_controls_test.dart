import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:qlaunch/main.dart' as app;
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/home_screen.dart';
import 'package:qlaunch/src/ui/settings_screen.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized().framePolicy =
      LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;

  testWidgets('saved duration controls tab clicks and swipe settling', (
    tester,
  ) async {
    await app.main();
    await tester.pumpAndSettle();
    final home = tester.widget<HomeScreen>(find.byType(HomeScreen));
    final state = home.state;
    final original = state.snapshot!.settings;
    addTearDown(() async {
      await state.updateSettings(
        SettingsPatch(
          pageTransitionMs: original.pageTransitionMs,
          continuousScroll: original.continuousScroll,
          tabVisibility: original.tabVisibility,
        ),
      );
    });
    await state.updateSettings(
      const SettingsPatch(
        continuousScroll: false,
        tabVisibility: TabVisibility.always,
      ),
    );
    await tester.pumpAndSettle();
    expect(state.pageIds.length, greaterThan(1));

    final context = tester.element(find.byType(HomeScreen));
    unawaited(
      Navigator.of(context).push<void>(
        MaterialPageRoute(builder: (_) => SettingsScreen(state: state)),
      ),
    );
    await tester.pumpAndSettle();
    final sliderFinder = find.byKey(
      SettingsScreen.controlKey(SettingField.pageTransitionMs),
    );
    await tester.ensureVisible(sliderFinder);
    await tester.pumpAndSettle();
    final slider = tester.widget<Slider>(sliderFinder);
    slider.onChanged!(800);
    slider.onChangeEnd!(800);
    await tester.pumpAndSettle();
    await state.load();
    expect(state.snapshot!.settings.pageTransitionMs.toInt(), 800);
    Navigator.of(context).pop();
    await tester.pumpAndSettle();

    double page() =>
        tester.widget<PageView>(find.byType(PageView)).controller!.page!;
    final start = page();
    await tester.tap(find.byKey(TabStrip.tapKey(1)).hitTestable().first);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 150));
    final intermediate = page();
    expect(intermediate, allOf(greaterThan(start), lessThan(start + 1)));
    await tester.pumpAndSettle();
    expect(page(), start + 1);

    final pager = find.byType(GroupPager);
    final width = tester.getSize(pager).width;
    final gesture = await tester.startGesture(
      tester.getCenter(pager) + Offset(width * .3, 0),
    );
    await gesture.moveBy(const Offset(-25, 0));
    await gesture.moveBy(Offset(-width * .6, 0));
    await tester.pump();
    final dragged = page();
    expect(dragged, allOf(greaterThan(start + 1), lessThan(start + 2)));
    await gesture.up();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 150));
    expect(page(), allOf(greaterThan(dragged), lessThan(start + 2)));
    await tester.pumpAndSettle();
    expect(page(), start + 2);
    debugPrint(
      'PAGE_TRANSITION_VERIFIED duration=800ms tabIntermediate=$intermediate swipeIntermediate=$dragged',
    );

    // The host driver sends real Android Back presses, exercising the JNI event
    // path as well as Dart's route/search/tab priority.
    var request = 0;
    Future<void> nativeBack() async {
      final before = state.backRequests;
      final id = request++;
      for (
        var attempt = 0;
        attempt < 40 && state.backRequests == before;
        attempt++
      ) {
        debugPrint('PAGE_CONTROL_BACK $id');
        await tester.pump(const Duration(milliseconds: 100));
      }
      expect(
        state.backRequests,
        greaterThan(before),
        reason: 'host must send native Back',
      );
      await tester.pumpAndSettle();
    }

    final tab = tester.widget<TabStrip>(find.byType(TabStrip));
    final ids = state.pageIds;
    var index = ids.indexOf(tab.currentGroupId);
    for (var i = 0; i <= ids.length; i++) {
      index = (index - 1) % ids.length;
      await nativeBack();
      expect(
        tester.widget<TabStrip>(find.byType(TabStrip)).currentGroupId,
        ids[index],
      );
    }
    final beforeDialog = page();
    unawaited(
      showDialog<void>(
        context: context,
        builder: (_) => const AlertDialog(title: Text('Back priority test')),
      ),
    );
    await tester.pumpAndSettle();
    await nativeBack();
    expect(find.byType(AlertDialog), findsNothing);
    expect(page(), beforeDialog);
    debugPrint('PAGE_BACK_VERIFIED presses=$request visibleTabs=${ids.length}');
  });
}
