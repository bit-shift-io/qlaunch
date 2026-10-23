import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/raster_cache_identity.dart';
import 'package:qlaunch/src/ui/raster_tile.dart' show RasterCacheKey;

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  _keyTests();

  testWidgets(
    'pixel environments miss while navigation-independent settings reuse',
    (tester) async {
      Future<Object> generation({
        Settings settings = testSettings,
        double dpr = 2,
        double scale = 1,
        TextDirection direction = TextDirection.ltr,
        Size size = const Size(600, 960),
        Locale locale = const Locale('en', 'US'),
      }) async {
        late Object result;
        await tester.pumpWidget(
          MaterialApp(
            home: Builder(
              builder: (context) => Localizations.override(
                context: context,
                locale: locale,
                child: MediaQuery(
                  data: MediaQueryData(
                    size: size,
                    devicePixelRatio: dpr,
                    textScaler: TextScaler.linear(scale),
                  ),
                  child: Directionality(
                    textDirection: direction,
                    child: Builder(
                      builder: (context) {
                        result = rasterGeneration(context, settings);
                        return const SizedBox.shrink();
                      },
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        return result;
      }

      final baseline = await generation();
      expect(
        await generation(settings: settingsWith(wallpaperAlpha: 50)),
        baseline,
      );
      expect(
        await generation(settings: settingsWith(continuousScroll: true)),
        baseline,
      );
      expect(
        await generation(settings: settingsWith(pageTransitionMs: 800)),
        baseline,
      );
      for (final settings in [
        settingsWith(labelShadowBlur: 5),
        settingsWith(labelSize: 20),
        settingsWith(labelTypeface: Typeface.monospace),
        settingsWith(labelColour: LabelColour.black),
        settingsWith(iconAlpha: 40),
        settingsWith(iconSize: 64),
        settingsWith(gridColumnsPortrait: 3),
        settingsWith(layout: LayoutStyle.list),
        settingsWith(iconsVisible: false),
      ]) {
        expect(await generation(settings: settings), isNot(baseline));
      }
      expect(await generation(dpr: 3), isNot(baseline));
      expect(await generation(scale: 1.5), isNot(baseline));
      expect(await generation(direction: TextDirection.rtl), isNot(baseline));
      expect(
        await generation(locale: const Locale('en', 'GB')),
        isNot(baseline),
      );
      expect(await generation(size: const Size(960, 600)), isNot(baseline));
    },
  );

  testWidgets(
    'shared geometry constrains icons without a second text measurement',
    (tester) async {
      late AppCellGeometry geometry;
      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) {
              geometry = appCellGeometry(
                context,
                testSettings,
                const BoxConstraints.tightFor(width: 100, height: 60),
                measuredLabelHeight: 20,
              );
              return const SizedBox.shrink();
            },
          ),
        ),
      );
      expect(geometry.cellSize, const Size(100, 60));
      expect(geometry.iconSide, 30);
      expect(geometry.labelHeight, 20);
    },
  );

  testWidgets(
    'item moves preserve pixel identity; icon and geometry changes miss',
    (tester) async {
      late AppCellGeometry geometry;
      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) {
              geometry = appCellGeometry(
                context,
                testSettings,
                const BoxConstraints.tightFor(width: 100, height: 100),
                measuredLabelHeight: 24,
              );
              return const SizedBox.shrink();
            },
          ),
        ),
      );
      final item = makeItem('a', 'home');
      final moved = makeItem('a', 'apps', order: 10);
      final a = appCellRasterKey(
        generation: 1,
        item: item,
        icon: null,
        settings: testSettings,
        geometry: geometry,
        kind: 'tile',
      );
      expect(
        appCellRasterKey(
          generation: 1,
          item: moved,
          icon: null,
          settings: testSettings,
          geometry: geometry,
          kind: 'tile',
        ),
        a,
      );
      expect(
        appCellRasterKey(
          generation: 1,
          item: makeItem('a', 'home', uninstalled: true),
          icon: null,
          settings: testSettings,
          geometry: geometry,
          kind: 'tile',
        ),
        isNot(a),
      );
      expect(a.owner, 'item:a');
    },
  );
}

void _keyTests() {
  const geometry = AppCellGeometry(
    cellSize: Size(100, 60),
    iconSide: 30,
    labelHeight: 20,
    devicePixelRatio: 2,
    scaledLabelSize: 12,
  );
  RasterCacheKey key(
    Item item, {
    ImageProvider? icon,
    AppCellGeometry g = geometry,
    String kind = 'tile',
  }) => appCellRasterKey(
    generation: 'g',
    item: item,
    icon: icon,
    settings: testSettings,
    geometry: g,
    kind: kind,
  );
  final item = makeItem('a', 'home');

  test('moving an unchanged item preserves every variant', () {
    final moved = makeItem('a', 'apps', order: 10);
    for (final kind in ['tile', 'icon', 'label']) {
      expect(key(moved, kind: kind), key(item, kind: kind));
    }
  });

  test('renaming changes label and tile, not icon pixels', () {
    final renamed = makeItem('a', 'home', label: 'Maps 2');
    expect(key(renamed, kind: 'label'), isNot(key(item, kind: 'label')));
    expect(key(renamed), isNot(key(item)));
    expect(key(renamed, kind: 'icon'), key(item, kind: 'icon'));
  });

  test('a replaced icon changes icon and tile, not label pixels', () {
    final a = MemoryImage(Uint8List.fromList([1]));
    final b = MemoryImage(Uint8List.fromList([2]));
    expect(
      key(item, icon: a, kind: 'icon'),
      isNot(key(item, icon: b, kind: 'icon')),
    );
    expect(key(item, icon: a), isNot(key(item, icon: b)));
    expect(
      key(item, icon: a, kind: 'label'),
      key(item, icon: b, kind: 'label'),
    );
  });

  test('actual geometry variants stay distinct', () {
    const other = AppCellGeometry(
      cellSize: Size(80, 60),
      iconSide: 30,
      labelHeight: 20,
      devicePixelRatio: 2,
      scaledLabelSize: 12,
    );
    expect(key(item, kind: 'label'), isNot(key(item, g: other, kind: 'label')));
    expect(key(item), isNot(key(item, g: other)));
    expect(key(item, kind: 'icon'), key(item, g: other, kind: 'icon'));
  });

  test('tab and heading keys share an owner but not a kind', () {
    final tab = labelRasterKey(
      generation: 'g',
      kind: 'tab',
      owner: 'group:home',
      pixels: 'Home',
    );
    final heading = labelRasterKey(
      generation: 'g',
      kind: 'heading',
      owner: 'group:home',
      pixels: 'Home',
    );
    expect(tab.owner, heading.owner);
    expect(tab, isNot(heading));
  });
}
