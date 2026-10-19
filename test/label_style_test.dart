import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/label_style.dart';

import 'fake_backend.dart';

void main() {
  test('uses the size, typeface, colour and shadow from the settings', () {
    final style = labelTextStyle(
      settingsWith(
        labelSize: 20,
        labelTypeface: Typeface.serif,
        labelColour: LabelColour.black,
        labelShadow: true,
      ),
    );

    expect(style.fontSize, 20);
    expect(style.fontFamily, 'serif');
    expect(style.color, Colors.black);
    // Black text gets a light halo, since a dark one would only smudge it.
    expect(style.shadows, isNotNull);
    expect(style.shadows!.single.color, Colors.white);
  });

  test('drops the shadow when the setting is off', () {
    expect(labelTextStyle(settingsWith(labelShadow: false)).shadows, isNull);
  });

  test('maps the typeface names the way the app labels do', () {
    expect(
      labelTextStyle(settingsWith(labelTypeface: Typeface.normal)).fontFamily,
      isNull,
    );
    expect(
      labelTextStyle(settingsWith(labelTypeface: Typeface.sans)).fontFamily,
      'sans-serif',
    );
    expect(
      labelTextStyle(
        settingsWith(labelTypeface: Typeface.monospace),
      ).fontFamily,
      'monospace',
    );
  });

  test('maps the label colours', () {
    expect(
      labelTextStyle(settingsWith(labelColour: LabelColour.white)).color,
      Colors.white,
    );
    expect(
      labelTextStyle(settingsWith(labelColour: LabelColour.grey)).color,
      Colors.grey,
    );
  });
}
