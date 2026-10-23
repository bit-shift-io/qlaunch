import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/advanced_screen.dart';
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
    testWidgets(
      'transition duration slider saves only milliseconds in range 100-800',
      (tester) async {
        await pumpSettings(tester);
        final control = find.byKey(
          const ValueKey('setting.pageTransitionMs.control'),
        );
        expect(control, findsOneWidget);
        await tester.ensureVisible(control);
        final slider = tester.widget<Slider>(control);
        expect(slider.min, 100);
        expect(slider.max, 800);
        expect(slider.value, 300);
        slider.onChanged!(650);
        slider.onChangeEnd!(650);
        await tester.pumpAndSettle();
        expect(fake.settingsPatches, [
          const SettingsPatch(pageTransitionMs: 650),
        ]);
      },
    );
    testWidgets('continuous scrolling sends only its mode setting', (
      tester,
    ) async {
      await pumpSettings(tester);
      await tapControl(
        tester,
        find.byKey(SettingsScreen.controlKey(SettingField.continuousScroll)),
      );
      expect(fake.settingsPatches, [
        const SettingsPatch(continuousScroll: true),
      ]);
    });

    testWidgets('every setting in the table has one', (tester) async {
      await pumpSettings(tester);
      expect(SettingField.values, hasLength(19));
      for (final field in SettingField.values) {
        expect(
          find.byKey(SettingsScreen.controlKey(field)),
          findsOneWidget,
          reason: 'no control for $field',
        );
      }
      expect(find.byKey(SettingsScreen.advancedKey), findsOneWidget);
    });

    testWidgets('continuous scrolling is no longer labelled a prototype', (
      tester,
    ) async {
      await pumpSettings(tester);

      expect(find.text('Continuous scrolling'), findsOneWidget);
      expect(find.text('Continuous scrolling (prototype)'), findsNothing);
    });

    testWidgets('shadow blur sends only that setting and spans 0-10', (
      tester,
    ) async {
      await pumpSettings(tester);
      final control = find.byKey(
        SettingsScreen.controlKey(SettingField.labelShadowBlur),
      );
      await tester.ensureVisible(control);
      await tester.pump();
      final slider = tester.widget<Slider>(control);
      expect(slider.value, testSettings.labelShadowBlur.toDouble());
      expect(slider.min, 0);
      expect(slider.max, 10);

      await tester.drag(control, const Offset(500, 0));
      await tester.pumpAndSettle();

      expect(fake.settingsPatches, hasLength(1));
      expect(fake.settingsPatches.single.labelShadowBlur, 10);
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
        tester.renderObject<RenderParagraph>(title).text.style?.fontSize,
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

    testWidgets('changing the scroll fade sends only that setting', (
      tester,
    ) async {
      await pumpSettings(tester);
      await tapControl(
        tester,
        find.byKey(
          SettingsScreen.choiceKey(
            SettingField.scrollFade,
            ScrollFadeStyle.tile,
          ),
        ),
      );
      expect(fake.settingsPatches, [
        const SettingsPatch(scrollFade: ScrollFadeStyle.tile),
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
      await tester.ensureVisible(find.byKey(SettingsScreen.advancedKey));
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

  testWidgets('the Advanced row opens the advanced screen', (tester) async {
    await pumpSettings(tester);

    await tapControl(tester, find.byKey(SettingsScreen.advancedKey));

    expect(find.byType(AdvancedScreen), findsOneWidget);
  });
}
