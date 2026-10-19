import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/group_editor_screen.dart';
import 'package:qlaunch/src/ui/settings_screen.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() async {
    fake = FakeBackend(
      snapshot: snapshotOf(pageIds: ['home'], groups: [makeGroup('home')]),
    );
    state = LauncherState(fake);
    await state.load();
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pumpSettings(WidgetTester tester) async {
    await tester.pumpWidget(MaterialApp(home: SettingsScreen(state: state)));
    await tester.pump();
  }

  Future<void> tapControl(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pump();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  group('controls', () {
    testWidgets('every setting in the table has one', (tester) async {
      await pumpSettings(tester);
      // §8.6's table: 15 settings plus the two actions below them.
      expect(SettingField.values, hasLength(15));
      for (final field in SettingField.values) {
        expect(
          find.byKey(SettingsScreen.controlKey(field)),
          findsOneWidget,
          reason: 'no control for $field',
        );
      }
      expect(find.byKey(SettingsScreen.restartKey), findsOneWidget);
      expect(find.byKey(SettingsScreen.factoryResetKey), findsOneWidget);
    });

    testWidgets('the text size row is called "Font size"', (tester) async {
      await pumpSettings(tester);

      // One "Font size" drives the app labels and the tab strip alike, so it
      // is no longer named for labels only.
      expect(find.text('Font size'), findsOneWidget);
      expect(find.text('Label size'), findsNothing);
    });

    testWidgets('the rows take the Font size', (tester) async {
      await pumpSettings(tester);

      // A list row's title uses the theme's body text, which the Font size
      // setting scales; unset it and this would be the 16 of the base theme.
      final title = find.descendant(
        of: find.byKey(SettingsScreen.groupsKey),
        matching: find.text('Groups'),
      );
      expect(
        tester
            .renderObject<RenderParagraph>(title)
            .text
            .style
            ?.fontSize,
        testSettings.labelSize.toDouble(),
      );
    });

    testWidgets('they open showing the snapshot\'s values', (tester) async {
      await pumpSettings(tester);

      expect(
        tester
            .widget<ChoiceChip>(
              find.byKey(
                SettingsScreen.choiceKey(
                  SettingField.layout,
                  testSettings.layout,
                ),
              ),
            )
            .selected,
        isTrue,
      );
      expect(
        tester
            .widget<Slider>(
              find.byKey(SettingsScreen.controlKey(SettingField.labelSize)),
            )
            .value,
        testSettings.labelSize.toDouble(),
      );
      expect(
        tester
            .widget<SwitchListTile>(
              find.byKey(SettingsScreen.controlKey(SettingField.labelsVisible)),
            )
            .value,
        testSettings.labelsVisible,
      );
    });

    testWidgets('changing the layout sends a patch holding only the layout', (
      tester,
    ) async {
      await pumpSettings(tester);
      await tapControl(
        tester,
        find.byKey(
          SettingsScreen.choiceKey(SettingField.layout, LayoutStyle.list),
        ),
      );
      // Equality on the whole patch is the exhaustive check that no other
      // field was sent with it.
      expect(fake.settingsPatches, [
        const SettingsPatch(layout: LayoutStyle.list),
      ]);
    });

    testWidgets('a value is shown as the snapshot returns it, not as set', (
      tester,
    ) async {
      await pumpSettings(tester);
      final control = find.byKey(
        SettingsScreen.controlKey(SettingField.iconsVisible),
      );
      expect(tester.widget<SwitchListTile>(control).value, isTrue);

      // The core is asked to hide icons and answers with a snapshot that still
      // shows them, as a core that refused the change would.
      await tapControl(tester, control);

      expect(fake.settingsPatches, [const SettingsPatch(iconsVisible: false)]);
      expect(tester.widget<SwitchListTile>(control).value, isTrue);
    });

    testWidgets(
      'a number the core does not keep comes back as the core has it',
      (tester) async {
        await pumpSettings(tester);
        final control = find.byKey(
          SettingsScreen.controlKey(SettingField.iconSize),
        );
        await tester.ensureVisible(control);
        await tester.pump();
        final slider = tester.widget<Slider>(control);

        // Drag well past the maximum of 100; the core keeps 48.
        await tester.drag(control, const Offset(500, 0));
        await tester.pumpAndSettle();

        expect(fake.settingsPatches, hasLength(1));
        expect(fake.settingsPatches.single.iconSize, isNotNull);
        expect(slider.value, testSettings.iconSize.toDouble());
        expect(tester.widget<Slider>(control).value, slider.value);
      },
    );
    testWidgets('the whole screen lays out on a narrow phone', (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(360, 640);
      addTearDown(tester.view.reset);
      await pumpSettings(tester);
      await tester.ensureVisible(find.byKey(SettingsScreen.factoryResetKey));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });
  });

  testWidgets('the Choose wallpaper row opens the system chooser', (
    tester,
  ) async {
    await pumpSettings(tester);
    fake.calls.clear();

    await tapControl(tester, find.byKey(SettingsScreen.chooseWallpaperKey));

    expect(fake.calls, contains('chooseWallpaper'));
  });

  testWidgets('the Groups row opens the group editor', (tester) async {
    await pumpSettings(tester);

    await tapControl(tester, find.byKey(SettingsScreen.groupsKey));

    expect(find.byType(GroupEditorScreen), findsOneWidget);
  });

  group('actions', () {
    testWidgets('Restart Launcher reads the model again', (tester) async {
      await pumpSettings(tester);
      final before = fake.calls.where((c) => c == 'init').length;
      await tapControl(tester, find.byKey(SettingsScreen.restartKey));
      expect(fake.calls.where((c) => c == 'init').length, before + 1);
    });

    testWidgets('Factory Reset asks first, and calls nothing', (tester) async {
      await pumpSettings(tester);
      final calls = List.of(fake.calls);
      final control = find.byKey(SettingsScreen.factoryResetKey);
      await tester.ensureVisible(control);
      await tester.pump();
      await tester.tap(control);
      await tester.pumpAndSettle();

      expect(find.textContaining('cannot be undone'), findsOneWidget);
      expect(fake.calls, calls);

      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(find.textContaining('cannot be undone'), findsNothing);
      expect(fake.calls, calls);
    });

    testWidgets('confirming says the core command is still missing', (
      tester,
    ) async {
      await pumpSettings(tester);
      final calls = List.of(fake.calls);
      final control = find.byKey(SettingsScreen.factoryResetKey);
      await tester.ensureVisible(control);
      await tester.pump();
      await tester.tap(control);
      await tester.pumpAndSettle();

      await tester.tap(find.text('Reset'));
      await tester.pumpAndSettle();

      expect(find.textContaining('not available yet'), findsOneWidget);
      expect(fake.calls, calls);
    });
  });
}
