import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/group_pager.dart';
import 'package:qlaunch/src/ui/tab_strip.dart';
import 'snapshot_builders.dart';

void main() {
  testWidgets('current title is centred and neighbours wrap and move with drag', (tester) async {
    final controller = GroupPagerController(pageIds: ['a', 'b', 'c', 'd']);
    final steps = <int>[];
    await tester.pumpWidget(MaterialApp(home: Scaffold(body: Align(alignment: Alignment.topLeft,
      child: SizedBox(width: 600, child: TabStrip(pageIds: controller.pageIds,
        groups: [for (final id in controller.pageIds) makeGroup(id)],
        currentGroupId: 'a', settings: testSettings, controller: controller, onStep: steps.add)),
    ))));
    expect(tester.getCenter(find.byKey(TabStrip.tabKey(0))).dx, 300);
    expect(tester.widget<Text>(find.byKey(TabStrip.tabKey(-1))).data, 'd');
    expect(tester.widget<Text>(find.byKey(TabStrip.tabKey(1))).data, 'b');
    await tester.tap(find.byKey(TabStrip.tabKey(-1)));
    expect(steps, [-1]);
    controller.position.value += 0.5;
    await tester.pump();
    expect(tester.getCenter(find.byKey(TabStrip.tabKey(0))).dx, 200);
    expect(tester.getCenter(find.byKey(TabStrip.tabKey(1))).dx, 400);
    expect(find.byType(SingleChildScrollView), findsNothing);
    controller.dispose();
  });

  testWidgets('one group has a single title', (tester) async {
    await tester.pumpWidget(MaterialApp(home: TabStrip(pageIds: const ['a'],
      groups: [makeGroup('a')], currentGroupId: 'a', settings: testSettings)));
    expect(find.text('a'), findsOneWidget);
  });
}
