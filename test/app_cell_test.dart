import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/app_cell.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

// A 1x1 PNG.
final Uint8List _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
);

const _app = AppRef(package: 'org.example.app', activity: 'Main');

Item _item({
  String label = 'Maps',
  bool uninstalled = false,
  AppRef? app,
  String? folderGroupId,
}) => Item(
  id: 'i1',
  groupId: 'g',
  label: label,
  labelOverridden: false,
  app: app,
  uninstalled: uninstalled,
  order: 0,
  folderGroupId: folderGroupId,
);

Future<void> _pumpCell(
  WidgetTester tester, {
  Item? item,
  Uint8List? icon,
  Settings? settings,
  VoidCallback? onTap,
  VoidCallback? onLongPress,
  bool folder = false,
  Size cell = const Size(100, 100),
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Center(
          child: SizedBox.fromSize(
            size: cell,
            child: AppCell(
              item: item ?? _item(app: _app),
              icon: icon == null ? null : MemoryImage(icon),
              settings: settings ?? testSettings,
              folder: folder,
              onTap: onTap,
              onLongPress: onLongPress,
            ),
          ),
        ),
      ),
    ),
  );
}

Rect get _cellRect => const Rect.fromLTWH(0, 0, 100, 100);

void main() {
  final iconFinder = find.byKey(AppCell.iconKey);
  final labelFinder = find.byKey(AppCell.labelKey);

  // The cell is centred in the test surface, so work in cell coordinates.
  Rect rectOf(WidgetTester tester, Finder f, Finder cell) =>
      tester.getRect(f).shift(-tester.getTopLeft(cell));

  group('layouts', () {
    testWidgets('tiles layout stacks icon above label', (tester) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(layout: LayoutStyle.tiles),
      );
      final cell = find.byType(AppCell);
      final icon = rectOf(tester, iconFinder, cell);
      final label = rectOf(tester, labelFinder, cell);
      expect(icon.bottom, lessThanOrEqualTo(label.top));
      expect(icon.center.dx, closeTo(label.center.dx, 0.5));
      expect(icon.size, const Size(48, 48));
      // Tiles sit at the top of the cell.
      expect(icon.top, 6);
    });

    testWidgets('list layout puts icon left of label on one line', (
      tester,
    ) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(layout: LayoutStyle.list, iconSize: 30),
      );
      final cell = find.byType(AppCell);
      final icon = rectOf(tester, iconFinder, cell);
      final label = rectOf(tester, labelFinder, cell);
      expect(icon.right, lessThanOrEqualTo(label.left));
      expect(icon.center.dy, closeTo(label.center.dy, 0.5));
    });

    testWidgets('centred layout does not overlap label and icon', (
      tester,
    ) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(layout: LayoutStyle.centred),
      );
      final cell = find.byType(AppCell);
      final icon = rectOf(tester, iconFinder, cell);
      final label = rectOf(tester, labelFinder, cell);
      expect(icon.overlaps(label), isFalse);
      expect(icon.bottom, lessThanOrEqualTo(label.top));
      // Centred as a whole, both ways.
      final both = icon.expandToInclude(label);
      expect(both.center.dy, closeTo(_cellRect.center.dy + 2, 0.5));
      expect(icon.center.dx, closeTo(_cellRect.center.dx, 0.5));
      expect(label.center.dx, closeTo(_cellRect.center.dx, 0.5));
    });

    testWidgets('a very large icon in a small cell does not overflow', (
      tester,
    ) async {
      for (final layout in LayoutStyle.values) {
        await _pumpCell(
          tester,
          icon: _png,
          cell: const Size(60, 60),
          settings: settingsWith(layout: layout, iconSize: 100, labelSize: 40),
        );
        expect(tester.takeException(), isNull, reason: '$layout');
        final cell = tester.getRect(find.byType(AppCell));
        expect(
          cell.contains(tester.getRect(iconFinder).center),
          isTrue,
          reason: '$layout',
        );
      }
    });
  });

  group('visibility', () {
    testWidgets('icons_visible=false hides the icon but keeps the label', (
      tester,
    ) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(iconsVisible: false),
      );
      expect(iconFinder, findsNothing);
      expect(find.byType(Image), findsNothing);
      expect(find.text('Maps'), findsOneWidget);
    });

    testWidgets('labels_visible=false hides the label but keeps the icon', (
      tester,
    ) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(labelsVisible: false),
      );
      expect(find.text('Maps'), findsNothing);
      expect(iconFinder, findsOneWidget);
    });

    for (final layout in LayoutStyle.values) {
      testWidgets('both hidden is legal in the $layout layout', (tester) async {
        await _pumpCell(
          tester,
          icon: _png,
          settings: settingsWith(
            layout: layout,
            iconsVisible: false,
            labelsVisible: false,
          ),
        );
        expect(tester.takeException(), isNull);
        expect(iconFinder, findsNothing);
        expect(find.byType(Text), findsNothing);
      });
    }

    testWidgets('size 0 for icon and label does not throw', (tester) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(iconSize: 0, labelSize: 0),
      );
      expect(tester.takeException(), isNull);
    });
  });

  group('icon', () {
    testWidgets('a folder with no icon draws a folder glyph', (tester) async {
      await _pumpCell(
        tester,
        item: _item(label: 'Chat', folderGroupId: 'f'),
        folder: true,
      );

      expect(find.byIcon(Icons.folder), findsOneWidget);
    });

    testWidgets('a folder tile is tappable', (tester) async {
      var taps = 0;
      await _pumpCell(
        tester,
        item: _item(label: 'Chat', folderGroupId: 'f'),
        folder: true,
        onTap: () => taps++,
      );

      await tester.tap(find.byType(AppCell));
      expect(taps, 1);
    });

    testWidgets('a null icon renders a placeholder and does not throw', (
      tester,
    ) async {
      await _pumpCell(tester);
      expect(tester.takeException(), isNull);
      expect(find.byKey(AppCell.placeholderKey), findsOneWidget);
      expect(find.byType(Image), findsNothing);
      expect(tester.getSize(iconFinder), const Size(48, 48));
    });

    testWidgets('icon bytes render as an image of icon_size dp', (
      tester,
    ) async {
      await _pumpCell(tester, icon: _png);
      expect(find.byType(Image), findsOneWidget);
      expect(find.byKey(AppCell.placeholderKey), findsNothing);
      expect(tester.getSize(iconFinder), const Size(48, 48));
      expect(tester.widget<Image>(find.byType(Image)).fit, BoxFit.contain);
    });

    testWidgets('bytes that are not an image fall back to the placeholder', (
      tester,
    ) async {
      await _pumpCell(tester, icon: Uint8List.fromList(const [1, 2, 3]));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump();
      // The decode failure is reported to the framework; it is handled here.
      tester.takeException();
      expect(find.byKey(AppCell.placeholderKey), findsOneWidget);
    });

    testWidgets('full opacity draws no Opacity layer', (tester) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(iconAlpha: 100),
      );
      expect(find.byType(Opacity), findsNothing);
    });

    testWidgets('a translucent alpha still fades', (tester) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(iconAlpha: 50),
      );
      expect(find.byType(Opacity), findsOneWidget);
    });

    testWidgets('icon_alpha reaches an Opacity over the icon', (tester) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(iconAlpha: 50),
      );
      final opacity = tester.widget<Opacity>(
        find.descendant(of: iconFinder, matching: find.byType(Opacity)),
      );
      expect(opacity.opacity, 0.5);
    });

    testWidgets('the placeholder is dimmed by icon_alpha too', (tester) async {
      await _pumpCell(tester, settings: settingsWith(iconAlpha: 20));
      final opacity = tester.widget<Opacity>(
        find.descendant(of: iconFinder, matching: find.byType(Opacity)),
      );
      expect(opacity.opacity, closeTo(0.2, 1e-9));
    });

    testWidgets('an out-of-range icon_alpha is clamped, not asserted', (
      tester,
    ) async {
      await _pumpCell(
        tester,
        icon: _png,
        settings: settingsWith(iconAlpha: 255),
      );
      expect(tester.takeException(), isNull);
      // 255 clamps to fully opaque, so no opacity layer is needed.
      expect(find.byType(Opacity), findsNothing);
    });
  });

  group('label', () {
    testWidgets('a very long unicode label is ellipsised, not overflowed', (
      tester,
    ) async {
      final long = '长い名前のアプリケーション🚀' * 20;
      for (final layout in LayoutStyle.values) {
        await _pumpCell(
          tester,
          item: _item(label: long, app: _app),
          icon: _png,
          settings: settingsWith(layout: layout),
        );
        expect(tester.takeException(), isNull, reason: '$layout');
        final text = tester.widget<Text>(labelFinder);
        expect(text.maxLines, 2);
        expect(text.overflow, TextOverflow.ellipsis);
        expect(
          tester.renderObject<RenderParagraph>(labelFinder).didExceedMaxLines,
          isTrue,
        );
        expect(
          tester.getRect(labelFinder).right,
          lessThanOrEqualTo(tester.getRect(find.byType(AppCell)).right),
          reason: '$layout',
        );
      }
    });

    testWidgets('label_size is the font size', (tester) async {
      await _pumpCell(tester, settings: settingsWith(labelSize: 11));
      expect(tester.widget<Text>(labelFinder).style!.fontSize, 11);
    });

    testWidgets('label_typeface maps to a font family', (tester) async {
      const expected = {
        Typeface.normal: null,
        Typeface.sans: 'sans-serif',
        Typeface.serif: 'serif',
        Typeface.monospace: 'monospace',
      };
      for (final entry in expected.entries) {
        await _pumpCell(
          tester,
          settings: settingsWith(labelTypeface: entry.key),
        );
        expect(
          tester.widget<Text>(labelFinder).style!.fontFamily,
          entry.value,
          reason: '${entry.key}',
        );
      }
    });

    testWidgets('label_colour maps to a colour', (tester) async {
      const expected = {
        LabelColour.white: Colors.white,
        LabelColour.black: Colors.black,
        LabelColour.grey: Colors.grey,
      };
      for (final entry in expected.entries) {
        await _pumpCell(tester, settings: settingsWith(labelColour: entry.key));
        expect(
          tester.widget<Text>(labelFinder).style!.color,
          entry.value,
          reason: '${entry.key}',
        );
      }
    });

    testWidgets('label_shadow adds a shadow, and off removes it', (
      tester,
    ) async {
      await _pumpCell(tester, settings: settingsWith(labelShadow: true));
      expect(tester.widget<Text>(labelFinder).style!.shadows, isNotEmpty);
      await _pumpCell(tester, settings: settingsWith(labelShadow: false));
      expect(tester.widget<Text>(labelFinder).style!.shadows, isNull);
    });
  });

  group('interaction', () {
    testWidgets('tapping an app calls onTap; long-press calls onLongPress', (
      tester,
    ) async {
      var taps = 0;
      var longs = 0;
      await _pumpCell(
        tester,
        icon: _png,
        onTap: () => taps++,
        onLongPress: () => longs++,
      );
      await tester.tap(find.byType(AppCell));
      expect(taps, 1);
      await tester.longPress(find.byType(AppCell));
      expect(longs, 1);
    });

    testWidgets('an item with no app does not tap, and does not throw', (
      tester,
    ) async {
      var taps = 0;
      await _pumpCell(tester, item: _item(), onTap: () => taps++);
      await tester.tap(find.byType(AppCell));
      expect(taps, 0);
      expect(tester.takeException(), isNull);
    });

    testWidgets('an uninstalled item does not tap, but can be long-pressed', (
      tester,
    ) async {
      var taps = 0;
      var longs = 0;
      await _pumpCell(
        tester,
        item: _item(app: _app, uninstalled: true),
        icon: _png,
        onTap: () => taps++,
        onLongPress: () => longs++,
      );
      await tester.tap(find.byType(AppCell));
      expect(taps, 0);
      await tester.longPress(find.byType(AppCell));
      expect(longs, 1);
    });

    testWidgets('an uninstalled item shows the placeholder, dimmed', (
      tester,
    ) async {
      await _pumpCell(
        tester,
        item: _item(app: _app, uninstalled: true),
        icon: _png,
      );
      expect(find.byKey(AppCell.placeholderKey), findsOneWidget);
      expect(find.byType(Image), findsNothing);
      expect(find.text('Maps'), findsOneWidget);
      final dim = tester.widget<Opacity>(
        find.ancestor(of: labelFinder, matching: find.byType(Opacity)),
      );
      expect(dim.opacity, lessThan(1));
    });

    testWidgets('long-press with no handler does nothing and does not throw', (
      tester,
    ) async {
      await _pumpCell(tester, icon: _png);
      await tester.longPress(find.byType(AppCell));
      await tester.tap(find.byType(AppCell));
      expect(tester.takeException(), isNull);
    });
  });

  group('cached mode', () {
    late RasterImageStore store;

    setUp(() {
      store = RasterImageStore()..activateGeneration(0);
      TileRasterCache.mode = TileRasterMode.combined;
    });

    tearDown(() {
      store.clear();
      TileRasterCache.reset();
    });

    Future<void> pumpCached(
      WidgetTester tester, {
      Item? item,
      Uint8List? icon,
      VoidCallback? onTap,
      VoidCallback? onLongPress,
      Settings? settings,
    }) async {
      await tester.pumpWidget(
        MaterialApp(
          home: RasterCacheScope(
            store: store,
            generation: 0,
            child: Scaffold(
              body: Center(
                child: SizedBox.fromSize(
                  size: const Size(100, 100),
                  child: AppCell(
                    item: item ?? _item(app: _app),
                    icon: icon == null ? null : MemoryImage(icon),
                    settings: settings ?? testSettings,
                    onTap: onTap,
                    onLongPress: onLongPress,
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(RawImage), findsOneWidget);
    }

    testWidgets('a captured cell keeps its tap and long-press behaviour', (
      tester,
    ) async {
      var taps = 0;
      var longs = 0;
      await pumpCached(
        tester,
        icon: _png,
        onTap: () => taps++,
        onLongPress: () => longs++,
      );
      await tester.tap(find.byType(AppCell));
      expect(taps, 1);
      await tester.longPress(find.byType(AppCell));
      expect(longs, 1);
    });

    testWidgets('a captured label keeps its accessibility text', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await pumpCached(tester, icon: _png);
      expect(find.bySemanticsLabel('Maps'), findsOneWidget);
      expect(find.byType(RawImage), findsOneWidget);
      handle.dispose();
    });

    testWidgets(
      'a broken icon is captured as the placeholder, not an empty tile',
      (tester) async {
        await pumpCached(tester, icon: Uint8List.fromList([1, 2, 3, 4]));
        expect(find.byType(RawImage), findsOneWidget);
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets('an uninstalled cell is still dimmed when captured', (
      tester,
    ) async {
      await pumpCached(
        tester,
        item: _item(app: _app, uninstalled: true),
        icon: _png,
      );
      final dim = tester.widget<Opacity>(
        find.ancestor(
          of: find.byType(RawImage),
          matching: find.byType(Opacity),
        ),
      );
      expect(dim.opacity, lessThan(1));
    });
  });
}
