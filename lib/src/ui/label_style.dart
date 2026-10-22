import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/settings.dart' hide Orientation;

/// The one place the "Font size" text settings become a [TextStyle].
///
/// App labels, the home tab strip and the lists all read them, so they cannot
/// drift apart: changing the size, typeface, colour or shadow moves every label
/// the user sees.
TextStyle labelTextStyle(Settings settings) {
  final colour = switch (settings.labelColour) {
    LabelColour.white => Colors.white,
    LabelColour.black => Colors.black,
    LabelColour.grey => Colors.grey,
  };
  return TextStyle(
    // Logical pixels. Flutter applies the user's text scale on top, which is
    // what makes this "sp".
    fontSize: settings.labelSize.toDouble(),
    fontFamily: labelFontFamily(settings),
    color: colour,
    shadows: settings.labelShadow
        ? [labelShadow(colour, blur: settings.labelShadowBlur.toDouble())]
        : null,
  );
}

/// The font family a typeface setting names, or `null` for the platform one.
String? labelFontFamily(Settings settings) => switch (settings.labelTypeface) {
  Typeface.normal => null,
  Typeface.sans => 'sans-serif',
  Typeface.serif => 'serif',
  Typeface.monospace => 'monospace',
};

/// A contrasting offset shadow keeps text readable over wallpaper.
///
/// `blur` is the user's Shadow blur setting. Blurred glyph masks are expensive
/// on Impeller/Vulkan (100+ ms raster frames paging warm labels on the Android
/// tablet), so it defaults to zero and only costs more when the user asks.
/// Rendering a whole tile to an image makes that cost a one-off capture.
Shadow labelShadow(Color colour, {double blur = 0}) => Shadow(
  color: colour == Colors.black ? Colors.white : Colors.black,
  offset: const Offset(1, 1),
  blurRadius: blur,
);

/// The theme's text, sized to the user's Font size and typeface.
///
/// The whole scale is multiplied by `label_size / body_large` so the primary
/// body text lands exactly on the chosen size while headings keep their
/// relative weight. Colour is left to the theme: unlike app labels and tabs,
/// lists sit on a surface rather than over the wallpaper.
ThemeData themeWithFontSettings(ThemeData base, Settings settings) {
  final body = base.textTheme.bodyLarge?.fontSize ?? 16;
  return base.copyWith(
    textTheme: base.textTheme.apply(
      fontFamily: labelFontFamily(settings),
      fontSizeFactor: settings.labelSize / body,
    ),
  );
}

/// Applies [themeWithFontSettings] to a subtree: the lists, the search panel
/// and the dialogs. A null [settings] (still loading) leaves the theme alone.
class LauncherFontScope extends StatelessWidget {
  const LauncherFontScope({
    super.key,
    required this.settings,
    required this.child,
  });

  final Settings? settings;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final settings = this.settings;
    if (settings == null) return child;
    return Theme(
      data: themeWithFontSettings(Theme.of(context), settings),
      child: child,
    );
  }
}
