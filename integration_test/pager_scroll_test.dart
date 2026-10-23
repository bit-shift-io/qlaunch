import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:qlaunch/main.dart' as app;
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/home_screen.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized().framePolicy =
      LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;

  testWidgets('Apps scrolls shortly after swiping away and back', (
    tester,
  ) async {
    await app.main();
    await tester.pumpAndSettle();
    final state = tester.widget<HomeScreen>(find.byType(HomeScreen)).state;
    expect(state.snapshot!.settings.continuousScroll, isFalse);
    final apps = state.appsGroup!.id;
    final tabs = tester.widget<TabStrip>(find.byType(TabStrip));
    tabs.onGroupTap!(apps);
    await tester.pumpAndSettle();

    final pager = find.byType(GroupPager);
    final width = tester.getSize(pager).width;
    for (var i = 0; i < 3; i++) {
      await tester.fling(pager, Offset(-width * 0.6, 0), 2400);
      await tester.pumpAndSettle();
      expect(
        tester.widget<TabStrip>(find.byType(TabStrip)).currentGroupId,
        isNot(apps),
      );
      await tester.fling(pager, Offset(width * 0.6, 0), 2400);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));
      expect(
        tester.widget<TabStrip>(find.byType(TabStrip)).currentGroupId,
        apps,
      );

      final vertical = find
          .byWidgetPredicate(
            (widget) =>
                widget is Scrollable &&
                axisDirectionToAxis(widget.axisDirection) == Axis.vertical,
          )
          .hitTestable();
      final position = tester.state<ScrollableState>(vertical).position;
      expect(position.maxScrollExtent, greaterThan(0));
      final before = position.pixels;
      await tester.drag(pager, const Offset(0, -100));
      await tester.pump();
      expect(position.pixels, greaterThan(before));
      await tester.pumpAndSettle();
    }
  });
}
