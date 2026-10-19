import 'package:flutter/material.dart';

/// A row of dots showing how many group pages there are and which is current.
///
/// It earns its place only when the tab strip cannot show every page at once
/// (`TabStrip.onOverflowChanged`); the home screen fades it in and out with
/// that, so a strip that already shows every page is not doubled up.
class PagePips extends StatelessWidget {
  const PagePips({super.key, required this.count, required this.current});

  /// How many pages there are, one dot each.
  final int count;

  /// The page the pager is showing, drawn as the bright dot.
  final int current;

  /// Where a dot sits. Keyed by index so a test can read one dot's colour.
  static Key dotKey(int index) => ValueKey('pagePips.$index');

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (var i = 0; i < count; i++)
          Container(
            key: dotKey(i),
            margin: const EdgeInsets.symmetric(horizontal: 3),
            width: 7,
            height: 7,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: Colors.white.withValues(alpha: i == current ? 0.9 : 0.35),
              // A shadow keeps the dots readable over a light wallpaper, the
              // same reason the labels carry one.
              boxShadow: const [
                BoxShadow(color: Colors.black54, blurRadius: 2),
              ],
            ),
          ),
      ],
    );
  }
}
