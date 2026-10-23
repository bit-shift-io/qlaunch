import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/group_pager.dart';

void main() {
  for (final milliseconds in [100, 800]) {
    testWidgets('swipe release follows configured ${milliseconds}ms duration', (
      tester,
    ) async {
      final controller = GroupPagerController(pageIds: ['a', 'b', 'c']);
      await tester.pumpWidget(
        MaterialApp(
          home: GroupPager(
            controller: controller,
            transitionDuration: Duration(milliseconds: milliseconds),
            gridFor: (_, id) => SizedBox.expand(key: ValueKey('grid.$id')),
          ),
        ),
      );
      final start = controller.position.value;
      final gesture = await tester.startGesture(const Offset(700, 300));
      await gesture.moveBy(const Offset(-30, 0));
      await gesture.moveBy(const Offset(-450, 0));
      await tester.pump();
      final dragged = controller.position.value;
      expect(dragged, allOf(greaterThan(start), lessThan(start + 1)));
      await gesture.up();
      await tester.pump();
      await tester.pump(Duration(milliseconds: milliseconds ~/ 2));
      expect(
        controller.position.value,
        allOf(greaterThan(dragged), lessThan(start + 1)),
      );
      await tester.pump(
        Duration(milliseconds: milliseconds - milliseconds ~/ 2),
      );
      expect(controller.position.value, start + 1);
      await tester.pumpAndSettle();
    });
  }

  testWidgets('reduced motion still animates explicit navigation', (
    tester,
  ) async {
    final controller = GroupPagerController(pageIds: ['a', 'b']);
    Widget host() => MaterialApp(
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(context).copyWith(disableAnimations: true),
        child: child!,
      ),
      home: GroupPager(controller: controller, gridFor: (_, id) => Text(id)),
    );
    await tester.pumpWidget(host());
    controller.step(-1);
    final target = (controller.initialPage - 1).toDouble();
    await tester.pumpWidget(host());
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 150));
    expect(
      controller.position.value,
      allOf(greaterThan(target), lessThan(controller.initialPage.toDouble())),
    );
    await tester.pumpAndSettle();
    expect(controller.position.value, target);
  });

  testWidgets(
    'grids crossfade with one column of travel and recover on release',
    (tester) async {
      final controller = GroupPagerController(pageIds: ['a', 'b', 'c']);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SizedBox(
              width: 400,
              child: GroupPager(
                controller: controller,
                columns: 4,
                gridFor: (_, id) => SizedBox.expand(key: ValueKey('grid.$id')),
              ),
            ),
          ),
        ),
      );
      final gesture = await tester.startGesture(const Offset(300, 300));
      await gesture.moveBy(const Offset(-20, 0));
      await gesture.moveBy(const Offset(-200, 0));
      await tester.pump();
      final a = find.byKey(const ValueKey('grid.a'));
      final opacity = tester.widget<Opacity>(
        find.ancestor(of: a, matching: find.byType(Opacity)).first,
      );
      expect(opacity.opacity, lessThanOrEqualTo(0.4));
      expect(tester.getTopLeft(a).dx, closeTo(-50, 10));
      await gesture.up();
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(
        controller.position.value,
        controller.position.value.roundToDouble(),
      );
    },
  );

  testWidgets('default explicit navigation takes 300ms instead of jumping', (
    tester,
  ) async {
    final controller = GroupPagerController(pageIds: ['a', 'b', 'c']);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 400,
            child: GroupPager(
              controller: controller,
              columns: 4,
              gridFor: (_, id) => SizedBox.expand(key: ValueKey('grid.$id')),
            ),
          ),
        ),
      ),
    );
    controller.goToGroup('b');
    final start = controller.initialPage.toDouble();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 400,
            child: GroupPager(
              controller: controller,
              columns: 4,
              gridFor: (_, id) => SizedBox.expand(key: ValueKey('grid.$id')),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 160));
    expect(
      tester.widget<PageView>(find.byType(PageView)).controller!.page,
      allOf(greaterThan(start), lessThan(start + 1)),
    );
    await tester.pump(const Duration(milliseconds: 140));
    expect(
      tester.widget<PageView>(find.byType(PageView)).controller!.page,
      controller.currentPage.toDouble(),
    );
    await tester.pumpAndSettle();
  });

  test('explicit navigation selects the nearest cycle copy', () {
    final controller = GroupPagerController(pageIds: ['a', 'b', 'c']);
    controller.goToGroup('c');
    expect(controller.currentPage, controller.initialPage - 1);
  });
}
