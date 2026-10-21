import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;
import 'package:qlaunch/src/ui/label_style.dart';

double gridLabelHeight(BuildContext context, Settings settings) {
  final painter = TextPainter(text: TextSpan(text: 'Ag\nAg', style: labelTextStyle(settings)),
    textDirection: Directionality.of(context), textScaler: MediaQuery.textScalerOf(context), maxLines: 2)
    ..layout();
  final textHeight = settings.labelsVisible ? painter.height : 0.0;
  painter.dispose();
  return textHeight;
}

double gridRowExtent(BuildContext context, Settings settings, double cellWidth) {
  final list = settings.layout == LayoutStyle.list;
  final icon = settings.iconsVisible
      ? math.min(settings.iconSize.toDouble(), math.max(0, list ? (cellWidth - 10) / 2 : cellWidth)) : 0.0;
  final textHeight = gridLabelHeight(context, settings);
  return math.max(24, list ? math.max(icon, textHeight) + 10
    : icon + textHeight + (icon > 0 && textHeight > 0 ? 2 : 0) + 8);
}
