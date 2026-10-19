import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/page_pips.dart';

void main() {
  Future<void> pumpPips(
    WidgetTester tester, {
    required int count,
    required int current,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: PagePips(count: count, current: current)),
      ),
    );
  }

  Color dotColour(WidgetTester tester, int index) {
    final box = tester.widget<Container>(find.byKey(PagePips.dotKey(index)));
    return (box.decoration! as BoxDecoration).color!;
  }

  testWidgets('draws one dot per page', (tester) async {
    await pumpPips(tester, count: 4, current: 0);

    for (var i = 0; i < 4; i++) {
      expect(find.byKey(PagePips.dotKey(i)), findsOneWidget);
    }
    expect(find.byKey(PagePips.dotKey(4)), findsNothing);
  });

  testWidgets('the current page\'s dot is brighter than the others', (
    tester,
  ) async {
    await pumpPips(tester, count: 3, current: 1);

    expect(
      dotColour(tester, 1).a,
      greaterThan(dotColour(tester, 0).a),
    );
  });
}
