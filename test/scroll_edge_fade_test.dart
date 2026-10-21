import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/scroll_edge_fade.dart';

void main() {
  testWidgets(
    'only scrollable content fades into the background at open edges',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final boundaryKey = GlobalKey();

      Future<void> show(double contentHeight) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Center(
              child: RepaintBoundary(
                key: boundaryKey,
                child: SizedBox(
                  width: 100,
                  height: 100,
                  child: ColoredBox(
                    color: Colors.blue,
                    child: ScrollEdgeFade(
                      child: ListView(
                        controller: controller,
                        padding: EdgeInsets.zero,
                        children: [
                          SizedBox(
                            height: contentHeight,
                            child: const ColoredBox(color: Colors.red),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
      }

      Future<List<int>> pixel(int y) async {
        final boundary =
            boundaryKey.currentContext!.findRenderObject()!
                as RenderRepaintBoundary;
        return (await tester.runAsync(() async {
          final image = await boundary.toImage(pixelRatio: 1);
          final bytes = (await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          final offset = (y * 100 + 50) * 4;
          final rgba = [for (var i = 0; i < 4; i++) bytes.getUint8(offset + i)];
          image.dispose();
          return rgba;
        }))!;
      }

      await show(500);
      expect(await pixel(1), [
        244,
        67,
        54,
        255,
      ]); // first tile stays solid at the start
      expect(
        (await pixel(98))[2],
        greaterThan(200),
      ); // bottom reveals blue background

      controller.jumpTo(200);
      await tester.pumpAndSettle();
      expect((await pixel(1))[2], greaterThan(200));
      expect((await pixel(98))[2], greaterThan(200));
      expect(await pixel(50), [244, 67, 54, 255]); // the middle is unaffected

      controller.jumpTo(controller.position.maxScrollExtent);
      await tester.pumpAndSettle();
      expect(await pixel(98), [
        244,
        67,
        54,
        255,
      ]); // last tile stays solid at the end

      controller.jumpTo(0);
      await show(50);
      expect(await pixel(1), [
        244,
        67,
        54,
        255,
      ]); // a short history page has no fade
    },
  );

  testWidgets('tile mode makes a whole tile transparent by half past an edge', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final boundaryKey = GlobalKey();

    await tester.pumpWidget(
      MaterialApp(
        home: Center(
          child: RepaintBoundary(
            key: boundaryKey,
            child: SizedBox(
              width: 100,
              height: 100,
              child: ColoredBox(
                color: Colors.blue,
                child: ListView(
                  controller: controller,
                  padding: EdgeInsets.zero,
                  children: [
                    for (var i = 0; i < 2; i++)
                      TileEdgeFade(
                        index: i,
                        columns: 1,
                        rowExtent: 100,
                        viewportHeight: 100,
                        child: const SizedBox(
                          height: 100,
                          child: ColoredBox(color: Colors.red),
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    Future<List<int>> pixel(int y) async {
      final boundary =
          boundaryKey.currentContext!.findRenderObject()!
              as RenderRepaintBoundary;
      return (await tester.runAsync(() async {
        final image = await boundary.toImage(pixelRatio: 1);
        final bytes = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        final offset = (y * 100 + 50) * 4;
        final rgba = [for (var i = 0; i < 4; i++) bytes.getUint8(offset + i)];
        image.dispose();
        return rgba;
      }))!;
    }

    expect(await pixel(1), const [244, 67, 54, 255]);
    expect(await pixel(98), const [244, 67, 54, 255]);

    // A quarter of the top tile is past the top edge: three quarters still
    // show, so it is drawn at half opacity. The tile below has only a quarter
    // left inside the viewport, so it is already fully transparent.
    controller.jumpTo(25);
    await tester.pumpAndSettle();
    final top = await pixel(1);
    expect(top[0], closeTo(138, 1));
    expect(top[1], closeTo(108, 1));
    expect(top[2], closeTo(148, 1));
    expect(await pixel(98), const [33, 150, 243, 255]);

    // Half of each tile is past an edge: tile mode is fully transparent then.
    controller.jumpTo(50);
    await tester.pumpAndSettle();
    expect(await pixel(1), const [33, 150, 243, 255]);
    expect(await pixel(98), const [33, 150, 243, 255]);

    controller.jumpTo(controller.position.maxScrollExtent);
    await tester.pumpAndSettle();
    expect(await pixel(98), const [244, 67, 54, 255]);
  });
}
