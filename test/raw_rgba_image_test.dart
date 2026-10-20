import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/raw_rgba_image.dart';

void main() {
  testWidgets('a 2x2 raw image paints without error', (tester) async {
    final rgba = Uint8List.fromList(List.filled(2 * 2 * 4, 0x80));
    await tester.pumpWidget(
      Image(image: RawRgbaImage(rgba, 2, 2), width: 2, height: 2),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('a short payload reaches the error builder, not a blank icon', (
    tester,
  ) async {
    final rgba = Uint8List.fromList(const [1, 2, 3, 4]);
    await tester.pumpWidget(
      MaterialApp(
        home: Image(
          image: RawRgbaImage(rgba, 2, 2),
          errorBuilder: (context, error, stack) => const Text('placeholder'),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('placeholder'), findsOneWidget);
  });

  test('equal bytes identity and size are equal keys', () {
    final rgba = Uint8List.fromList(List.filled(16, 0));
    expect(RawRgbaImage(rgba, 2, 2), RawRgbaImage(rgba, 2, 2));
  });
}
