import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/tab_strip.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  Future<void> pumpStrip(
    WidgetTester tester, {
    required List<String> pageIds,
    required String currentGroupId,
    List<Group>? groups,
    Settings settings = testSettings,
    double width = 1200,
    ValueChanged<bool>? onOverflow,
    ValueChanged<String>? onTap,
  }) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = Size(width, 160);
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topCenter,
            child: TabStrip(
              pageIds: pageIds,
              groups: groups ?? [for (final id in pageIds) makeGroup(id)],
              currentGroupId: currentGroupId,
              settings: settings,
              onGroupTap: onTap,
              onOverflowChanged: onOverflow,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  TextStyle tabStyle(WidgetTester tester, int index) {
    final text = tester.widget<Text>(
      find.descendant(
        of: find.byKey(TabStrip.tabKey(index)),
        matching: find.byType(Text),
      ),
    );
    return text.style!;
  }

  testWidgets('tab labels take their size, typeface, colour and shadow from '
      'the font settings', (tester) async {
    await pumpStrip(
      tester,
      pageIds: ['a', 'b'],
      currentGroupId: 'a',
      settings: settingsWith(
        labelSize: 20,
        labelTypeface: Typeface.serif,
        labelColour: LabelColour.black,
        labelShadow: false,
      ),
    );

    final style = tabStyle(tester, 0);
    expect(style.fontSize, 20);
    expect(style.fontFamily, 'serif');
    expect(style.color, Colors.black);
    expect(style.shadows, isNull);
  });

  testWidgets('the current tab is emphasised and the others are dimmed', (
    tester,
  ) async {
    await pumpStrip(
      tester,
      pageIds: ['a', 'b'],
      currentGroupId: 'a',
      settings: settingsWith(labelColour: LabelColour.white),
    );

    final current = tabStyle(tester, 0);
    final other = tabStyle(tester, 1);
    expect(current.fontWeight, FontWeight.w600);
    expect(other.fontWeight, FontWeight.w400);
    expect(current.color, Colors.white);
    // Same colour, less opaque, so the current tab stands out.
    expect(other.color!.a, lessThan(1));
    expect(current.color!.a, 1);
  });

  testWidgets('when every tab fits they all show, centred as a group, and '
      'nothing overflows', (tester) async {
    bool? overflow;
    await pumpStrip(
      tester,
      pageIds: ['a', 'b', 'c'],
      currentGroupId: 'a',
      onOverflow: (v) => overflow = v,
    );

    expect(overflow, isFalse);
    for (var i = 0; i < 3; i++) {
      expect(find.byKey(TabStrip.tabKey(i)), findsOneWidget);
    }
    // The middle of three equal tabs sits in the middle of the strip.
    expect(tester.getCenter(find.byKey(TabStrip.tabKey(1))).dx, closeTo(600, 2));
  });

  testWidgets('when the tabs overflow the selected one scrolls to the centre',
      (tester) async {
    bool? overflow;
    final ids = [for (var i = 0; i < 10; i++) 'group$i'];
    await pumpStrip(
      tester,
      pageIds: ids,
      currentGroupId: 'group5',
      width: 200,
      onOverflow: (v) => overflow = v,
    );

    expect(overflow, isTrue);
    expect(
      tester.getCenter(find.byKey(TabStrip.tabKey(5))).dx,
      closeTo(100, 2),
    );
  });

  testWidgets('the underline is as wide as its label', (tester) async {
    await pumpStrip(tester, pageIds: ['Apps', 'Home'], currentGroupId: 'Apps');

    final underline = tester
        .getSize(find.byKey(TabStrip.indicatorKey))
        .width;
    final label = tester.getSize(find.text('Apps')).width;

    expect(underline, closeTo(label, 0.5));
  });

  testWidgets('re-reports overflow when the width changes on its own', (
    tester,
  ) async {
    final seen = <bool>[];
    final ids = [for (var i = 0; i < 8; i++) 'group$i'];
    await pumpStrip(
      tester,
      pageIds: ids,
      currentGroupId: 'group0',
      width: 200,
      onOverflow: seen.add,
    );
    expect(seen.last, isTrue);

    // Widening keeps the same portrait orientation, so nothing rebuilds the
    // parent: the strip has to notice the new width itself.
    tester.view.physicalSize = const Size(1200, 160);
    await tester.pumpAndSettle();

    expect(seen.last, isFalse);
  });

  testWidgets('a tap still reports the group id', (tester) async {
    String? tapped;
    await pumpStrip(
      tester,
      pageIds: ['a', 'b'],
      currentGroupId: 'a',
      onTap: (id) => tapped = id,
    );

    await tester.tap(find.byKey(TabStrip.tabKey(1)));
    expect(tapped, 'b');
  });
}
