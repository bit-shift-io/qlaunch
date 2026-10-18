import 'dart:typed_data';

import 'package:flutter/material.dart';

/// The bottom layer of the home screen: the wallpaper, or a solid base colour
/// when there is none, under the black overlay whose opacity is
/// `wallpaper_alpha`.
///
/// §7.7: the wallpaper is drawn in Flutter rather than letting a translucent
/// surface show the OS wallpaper through, which is fragile. On Linux there is
/// no wallpaper for a windowed app, so the base is a solid colour and only the
/// overlay setting applies.
///
/// The bytes are a parameter because nothing in this plan fetches them: the
/// spec's `wallpaper_png()` is Android's and was not part of the bridge Task 14
/// built (F2).
class BackgroundLayer extends StatelessWidget {
  const BackgroundLayer({super.key, required this.alpha, this.wallpaper});

  /// `wallpaper_alpha`, 0-100: 0 shows the wallpaper alone, 100 is black.
  final int alpha;

  /// The wallpaper's PNG bytes, or `null` when there is none.
  final Uint8List? wallpaper;

  static const wallpaperKey = ValueKey('background.wallpaper');
  static const baseKey = ValueKey('background.base');

  /// The overlay. Its alpha is the setting, so a test can read it directly.
  static const overlayKey = ValueKey('background.overlay');

  /// Where there is no wallpaper to draw. A builder choice: a neutral dark
  /// reads as a background, where pure black reads as nothing having rendered.
  static const baseColour = Color(0xFF1B1B1F);

  @override
  Widget build(BuildContext context) {
    // Rust clamps the setting, but the overlay is the whole screen, so clamp
    // again rather than trust a value from outside.
    final overlay = (alpha / 100).clamp(0.0, 1.0);
    return Stack(
      fit: StackFit.expand,
      children: [
        if (wallpaper == null)
          const ColoredBox(key: baseKey, color: baseColour)
        else
          Image.memory(
            wallpaper!,
            key: wallpaperKey,
            fit: BoxFit.cover,
            gaplessPlayback: true,
            // A wallpaper that cannot be decoded must not take the screen
            // down with it; fall back to the base colour.
            errorBuilder: (context, error, stack) =>
                const ColoredBox(key: baseKey, color: baseColour),
          ),
        ColoredBox(
          key: overlayKey,
          color: Colors.black.withValues(alpha: overlay),
        ),
      ],
    );
  }
}
