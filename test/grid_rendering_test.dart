import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_cell.dart';
import 'package:qlaunch/src/ui/app_grid.dart';
import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  testWidgets('scaled two-line labels keep font size and rows fit their contents', (tester) async {
    final backend = FakeBackend(snapshot: snapshotOf(
      pageIds: ['g'], groups: [makeGroup('g')],
      items: [makeItem('A long application label that wraps', 'g')],
      settings: settingsWith(labelSize: 30, iconSize: 40),
    ));
    final state = LauncherState(backend);
    await state.load();
    await tester.pumpWidget(MaterialApp(home: MediaQuery(
      data: const MediaQueryData(size: Size(400, 800), textScaler: TextScaler.linear(1.5)),
      child: Scaffold(body: SizedBox(width: 400, child: AppGrid(state: state, groupId: 'g'))),
    )));
    await tester.pumpAndSettle();
    final label = tester.widget<Text>(find.byKey(AppCell.labelKey));
    expect(label.maxLines, 2);
    expect(label.softWrap, isTrue);
    expect(find.byType(FittedBox), findsNothing);
    expect(tester.getSize(find.byType(AppCell)).height, greaterThanOrEqualTo(140));
    expect(tester.getSize(find.byKey(AppCell.iconKey)).height, 40);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
    state.dispose();
    await backend.close();
  });
}
