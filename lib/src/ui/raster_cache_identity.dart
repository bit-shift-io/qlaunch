import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/grid_geometry.dart';
import 'package:qlaunch/src/ui/raster_tile.dart';

/// Pixel-affecting environment shared by every tile in one appearance.
///
/// Deliberately excludes paging position, animation alpha, usage counts, group
/// order and wallpaper alpha: none of them changes a captured tile.
Object rasterGeneration(BuildContext context, Settings settings) {
  final media = MediaQuery.of(context);
  final columns = math.max(
    1,
    (media.orientation == Orientation.portrait
            ? settings.gridColumnsPortrait
            : settings.gridColumnsLandscape)
        .toInt(),
  );
  return (
    TileRasterCache.mode,
    settings.layout,
    settings.iconsVisible,
    settings.iconSize,
    settings.iconAlpha,
    settings.labelsVisible,
    settings.labelSize,
    settings.labelTypeface,
    settings.labelColour,
    settings.labelShadow,
    settings.labelShadowBlur,
    columns,
    media.size,
    media.devicePixelRatio,
    media.padding,
    media.textScaler.scale(settings.labelSize.toDouble()),
    Localizations.localeOf(context),
    Directionality.of(context),
    DefaultTextStyle.of(context).style,
  );
}

/// The layout inputs of one cell, measured once per grid rather than per cell.
class AppCellGeometry {
  const AppCellGeometry({
    required this.cellSize,
    required this.iconSide,
    required this.labelHeight,
    required this.devicePixelRatio,
    required this.scaledLabelSize,
  });

  final Size cellSize;
  final double iconSide;
  final double labelHeight;
  final double devicePixelRatio;
  final double scaledLabelSize;

  @override
  bool operator ==(Object other) =>
      other is AppCellGeometry &&
      cellSize == other.cellSize &&
      iconSide == other.iconSide &&
      labelHeight == other.labelHeight &&
      devicePixelRatio == other.devicePixelRatio &&
      scaledLabelSize == other.scaledLabelSize;

  @override
  int get hashCode => Object.hash(
    cellSize,
    iconSide,
    labelHeight,
    devicePixelRatio,
    scaledLabelSize,
  );
}

AppCellGeometry appCellGeometry(
  BuildContext context,
  Settings settings,
  BoxConstraints constraints, {
  double? measuredLabelHeight,
}) {
  final layout = settings.layout;
  final width = constraints.hasBoundedWidth ? constraints.maxWidth : 100.0;
  final list = layout == LayoutStyle.list;
  final labelHeight = measuredLabelHeight ?? gridLabelHeight(context, settings);
  var side = settings.iconSize.toDouble();
  side = math.min(side, list ? math.max(0, (width - 10) / 2) : width);
  if (constraints.hasBoundedHeight) {
    side = math.min(
      side,
      math.max(0, constraints.maxHeight - (list ? 10 : labelHeight + 10)),
    );
  }
  final biggest = constraints.biggest;
  return AppCellGeometry(
    cellSize: Size(
      biggest.width.isFinite ? biggest.width : width,
      biggest.height.isFinite ? biggest.height : 0,
    ),
    iconSide: side,
    labelHeight: labelHeight,
    devicePixelRatio: MediaQuery.devicePixelRatioOf(context),
    scaledLabelSize: MediaQuery.textScalerOf(context)
        .scale(settings.labelSize.toDouble()),
  );
}

RasterCacheKey appCellRasterKey({
  required Object generation,
  required Item item,
  required ImageProvider? icon,
  required Settings settings,
  required AppCellGeometry geometry,
  required String kind,
}) {
  final provider = item.uninstalled ? null : icon;
  final layout = settings.layout;
  final geometryPixels = (
    geometry.cellSize,
    geometry.iconSide,
    geometry.labelHeight,
    geometry.devicePixelRatio,
    geometry.scaledLabelSize,
  );
  final labelPixels = (
    item.label,
    settings.labelsVisible,
    layout,
    settings.labelSize,
    settings.labelTypeface,
    settings.labelColour,
    settings.labelShadow,
    settings.labelShadowBlur,
    geometryPixels,
  );
  final iconPixels = (
    item.shortcut?.runtimeType,
    item.uninstalled,
    item.app?.package,
    item.app?.activity,
    provider,
    settings.iconsVisible,
    settings.iconAlpha,
    settings.iconSize,
    geometry.iconSide,
    geometry.devicePixelRatio,
  );
  final Object pixels = switch (kind) {
    'label' => labelPixels,
    'icon' => iconPixels,
    'tile' => (
      layout,
      iconPixels,
      labelPixels,
      settings.iconsVisible,
      settings.labelsVisible,
    ),
    _ => throw ArgumentError.value(
      kind,
      'kind',
      'Expected tile, icon or label',
    ),
  };
  return RasterCacheKey(
    generation: generation,
    kind: kind,
    owner: 'item:${item.id}',
    pixels: pixels,
  );
}

RasterCacheKey labelRasterKey({
  required Object generation,
  required String kind,
  required String owner,
  required Object pixels,
}) => RasterCacheKey(
  generation: generation,
  kind: kind,
  owner: owner,
  pixels: pixels,
);
