import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/background_layer.dart';

// A 1x1 PNG.
final Uint8List _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
);

Future<void> _pump(
  WidgetTester tester, {
  int alpha = 0,
  Uint8List? wallpaper,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: BackgroundLayer(alpha: alpha, wallpaper: wallpaper),
      ),
    ),
  );
}

void main() {
  Color overlay(WidgetTester tester) =>
      tester.widget<ColoredBox>(find.byKey(BackgroundLayer.overlayKey)).color;

  group('overlay', () {
    testWidgets('alpha 0 leaves the base layer fully visible', (tester) async {
      await _pump(tester, alpha: 0);
      expect(overlay(tester).a, 0.0);
    });

    testWidgets('alpha 100 is opaque black', (tester) async {
      await _pump(tester, alpha: 100);
      final colour = overlay(tester);
      expect(colour.a, 1.0);
      expect(colour.r, 0.0);
      expect(colour.g, 0.0);
      expect(colour.b, 0.0);
    });

    testWidgets('alpha 50 is half', (tester) async {
      await _pump(tester, alpha: 50);
      expect(overlay(tester).a, closeTo(0.5, 1e-9));
    });

    testWidgets('an out-of-range alpha is clamped, not asserted', (
      tester,
    ) async {
      await _pump(tester, alpha: 255);
      expect(tester.takeException(), isNull);
      expect(overlay(tester).a, 1.0);
    });

    testWidgets('a negative alpha is clamped', (tester) async {
      await _pump(tester, alpha: -3);
      expect(tester.takeException(), isNull);
      expect(overlay(tester).a, 0.0);
    });
  });

  group('base layer', () {
    testWidgets('no wallpaper draws the solid base colour', (tester) async {
      await _pump(tester);
      expect(find.byKey(BackgroundLayer.baseKey), findsOneWidget);
      expect(find.byType(Image), findsNothing);
      expect(tester.takeException(), isNull);
    });

    testWidgets('wallpaper bytes are drawn instead of the base colour', (
      tester,
    ) async {
      await _pump(tester, alpha: 0, wallpaper: _png);
      expect(find.byKey(BackgroundLayer.wallpaperKey), findsOneWidget);
      expect(find.byKey(BackgroundLayer.baseKey), findsNothing);
      expect(tester.widget<Image>(find.byType(Image)).fit, BoxFit.cover);
    });

    testWidgets('bytes that are not an image fall back to the base colour', (
      tester,
    ) async {
      await _pump(tester, wallpaper: Uint8List.fromList(const [1, 2, 3]));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump();
      // The decode failure is reported to the framework; it is handled here.
      tester.takeException();
      expect(find.byKey(BackgroundLayer.baseKey), findsOneWidget);
    });

    testWidgets('it covers the whole surface', (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(400, 800);
      addTearDown(tester.view.reset);
      await _pump(tester, alpha: 30);
      expect(
        tester.getSize(find.byType(BackgroundLayer)),
        const Size(400, 800),
      );
    });
  });
}
