import 'dart:math' as math;

import 'package:flutter/widgets.dart';

/// Fades only a vertical scrollable's content into its existing background.
/// Fixed titles, system bars and wallpaper remain outside the mask.
class ScrollEdgeFade extends StatefulWidget {
  const ScrollEdgeFade({super.key, required this.child});

  final Widget child;

  @override
  State<ScrollEdgeFade> createState() => _ScrollEdgeFadeState();
}

/// Fades each whole cell by how much of it is still inside the viewport.
///
/// Unlike [ScrollEdgeFade], which fades the pixels at the viewport edge, this
/// gives the icon and label of one cell the same opacity. A fully visible cell
/// is drawn at full opacity; a cell with only a quarter left inside is drawn at
/// half opacity; by the time half of it is past the edge it is fully
/// transparent.
class TileEdgeFade extends StatelessWidget {
  const TileEdgeFade({
    super.key,
    required this.index,
    required this.columns,
    required this.rowExtent,
    required this.viewportHeight,
    required this.child,
  });

  /// The cell's index in the grid, used to find its row.
  final int index;

  /// The grid's column count.
  final int columns;

  /// The height of one grid row, including its spacing.
  final double rowExtent;

  /// The viewport height the grid scrolls inside.
  final double viewportHeight;

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final position = Scrollable.of(context).position;
    return AnimatedBuilder(
      animation: position,
      child: child,
      builder: (context, child) {
        final top = (index ~/ columns) * rowExtent - position.pixels;
        final bottom = top + rowExtent;
        final visible = math.min(bottom, viewportHeight) - math.max(top, 0.0);
        final visibleFraction = (visible / rowExtent).clamp(0.0, 1.0);
        return Opacity(
          opacity: ((visibleFraction - 0.5) / 0.5).clamp(0.0, 1.0),
          child: child,
        );
      },
    );
  }
}

class _ScrollEdgeFadeState extends State<ScrollEdgeFade> {
  static const _fadeExtent = 32.0;
  final _edges = ValueNotifier<(double, double)>((0, 0));

  void _update(ScrollMetrics metrics, int depth) {
    if (depth != 0 || metrics.axis != Axis.vertical) return;
    // Ease the mask out near a real list boundary so the first/last row is
    // fully readable at rest. Content that fits never gets a mask at either end.
    final before = (metrics.extentBefore / _fadeExtent).clamp(0.0, 1.0);
    final after = (metrics.extentAfter / _fadeExtent).clamp(0.0, 1.0);
    _edges.value = metrics.axisDirection == AxisDirection.up
        ? (after, before)
        : (before, after);
  }

  @override
  void dispose() {
    _edges.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) =>
      NotificationListener<ScrollMetricsNotification>(
        onNotification: (notification) {
          _update(notification.metrics, notification.depth);
          return false;
        },
        child: NotificationListener<ScrollNotification>(
          onNotification: (notification) {
            _update(notification.metrics, notification.depth);
            return false;
          },
          child: ValueListenableBuilder<(double, double)>(
            valueListenable: _edges,
            child: widget.child,
            builder: (context, edges, child) => ShaderMask(
              blendMode: BlendMode.dstIn,
              shaderCallback: (bounds) {
                final fraction = bounds.height <= 0
                    ? 0.0
                    : math.min(_fadeExtent / bounds.height, 0.5);
                return LinearGradient(
                  begin: Alignment.topCenter,
                  end: Alignment.bottomCenter,
                  colors: [
                    const Color(0xFFFFFFFF).withValues(alpha: 1 - edges.$1),
                    const Color(0xFFFFFFFF),
                    const Color(0xFFFFFFFF),
                    const Color(0xFFFFFFFF).withValues(alpha: 1 - edges.$2),
                  ],
                  stops: [0, fraction, 1 - fraction, 1],
                ).createShader(bounds);
              },
              child: child,
            ),
          ),
        ),
      );
}
