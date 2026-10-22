import 'dart:io' show Platform;

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';

/// The bottom layer of the home screen: the wallpaper (or a solid base colour
/// where there is none), under the black overlay whose opacity is
/// `wallpaper_alpha`.
///
/// On Android the wallpaper is the system's own, shown through a transparent
/// window (`windowShowWallpaper` in the theme, `BackgroundMode.transparent` in
/// MainActivity) rather than read as bytes: Android 14+ refuses to hand an app
/// the wallpaper bytes without a media permission, which a launcher should not
/// need. So here the base is transparent on Android and the overlay dims the
/// system wallpaper underneath. On Linux there is no wallpaper for a windowed
/// app, so the base is a solid colour and only the overlay setting applies.
class BackgroundLayer extends StatelessWidget {
  const BackgroundLayer({super.key, required this.alpha});

  /// `wallpaper_alpha`, 0-100: 0 shows the wallpaper alone, 100 is black.
  final int alpha;

  static const baseKey = ValueKey('background.base');

  /// The overlay. Its alpha is the setting, so a test can read it directly.
  static const overlayKey = ValueKey('background.overlay');

  /// Where there is no wallpaper to draw. A builder choice: a neutral dark
  /// reads as a background, where pure black reads as nothing having rendered.
  static const baseColour = Color(0xFF1B1B1F);

  /// Whether the base is left transparent for the window's own wallpaper to
  /// show through. Android only; tests run elsewhere and keep the solid base.
  static bool get _transparentBase => !kIsWeb && Platform.isAndroid;

  @override
  Widget build(BuildContext context) {
    // Rust clamps the setting, but the overlay is the whole screen, so clamp
    // again rather than trust a value from outside.
    final overlay = (alpha / 100).clamp(0.0, 1.0);
    return Stack(
      fit: StackFit.expand,
      children: [
        ColoredBox(
          key: baseKey,
          color: _transparentBase
              ? Colors.transparent
              : BackgroundLayer.baseColour,
        ),
        ColoredBox(
          key: overlayKey,
          color: Colors.black.withValues(alpha: overlay),
        ),
      ],
    );
  }
}
