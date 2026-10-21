import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/group_pager.dart';

void main() {
  testWidgets('grids crossfade with one column of travel and recover on release', (tester) async {
    final controller = GroupPagerController(pageIds: ['a', 'b', 'c']);
    await tester.pumpWidget(MaterialApp(home: Scaffold(body: SizedBox(
      width: 400,
      child: GroupPager(controller: controller, columns: 4,
        gridFor: (_, id) => SizedBox.expand(key: ValueKey('grid.$id'))),
    ))));
    final gesture = await tester.startGesture(const Offset(300, 300));
    await gesture.moveBy(const Offset(-20, 0));
    await gesture.moveBy(const Offset(-200, 0));
    await tester.pump();
    final a = find.byKey(const ValueKey('grid.a'));
    final opacity = tester.widget<Opacity>(find.ancestor(of: a, matching: find.byType(Opacity)).first);
    expect(opacity.opacity, lessThanOrEqualTo(0.4));
    expect(tester.getTopLeft(a).dx, closeTo(-50, 10));
    await gesture.up();
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(controller.position.value, controller.position.value.roundToDouble());
  });

  testWidgets('explicit navigation settles within 160ms', (tester) async {
    final controller = GroupPagerController(pageIds: ['a', 'b', 'c']);
    await tester.pumpWidget(MaterialApp(home: Scaffold(body: SizedBox(
      width: 400,
      child: GroupPager(controller: controller, columns: 4,
        gridFor: (_, id) => SizedBox.expand(key: ValueKey('grid.$id'))),
    ))));
    controller.goToGroup('b');
    await tester.pumpWidget(MaterialApp(home: Scaffold(body: SizedBox(
      width: 400,
      child: GroupPager(controller: controller, columns: 4,
        gridFor: (_, id) => SizedBox.expand(key: ValueKey('grid.$id'))),
    ))));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 160));
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
