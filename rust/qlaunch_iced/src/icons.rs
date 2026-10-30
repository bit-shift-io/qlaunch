//! The bundled Material Icons font and the glyphs the UI draws.
//!
//! The font is Flutter's `MaterialIcons-Regular.otf`; its codepoints are
//! copied from `lib/src/material/icons.dart` so an icon matches the Flutter
//! build exactly.

use iced::Font;

/// The bundled icon font's family name, as stored in the OTF.
pub(crate) const FONT: Font = Font::with_name("Material Icons");

pub(crate) const SETTINGS: char = '\u{e57f}';
pub(crate) const ADD_LINK: char = '\u{e053}';
pub(crate) const CREATE_NEW_FOLDER: char = '\u{ef8c}';
pub(crate) const DASHBOARD_CUSTOMIZE: char = '\u{e1b2}';
pub(crate) const WALLPAPER: char = '\u{e6ca}';
pub(crate) const LOCK_OUTLINE: char = '\u{e3b1}';
pub(crate) const NOTIFICATIONS_NONE: char = '\u{e451}';
pub(crate) const LINK: char = '\u{e380}';
pub(crate) const CHECKLIST: char = '\u{e15b}';
pub(crate) const CHECK: char = '\u{e156}';
pub(crate) const ARROW_UPWARD: char = '\u{e0a0}';
pub(crate) const ARROW_DOWNWARD: char = '\u{e097}';
pub(crate) const ARROW_BACK: char = '\u{e092}';
pub(crate) const FOLDER: char = '\u{e2a3}';
pub(crate) const KEYBOARD_ARROW_DOWN: char = '\u{e353}';
pub(crate) const ADD: char = '\u{e047}';
pub(crate) const CLOSE: char = '\u{e16a}';

/// The glyph as owned text, for `text(...).font(FONT)`.
pub(crate) fn glyph(character: char) -> String {
    character.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_codepoints_match_flutter() {
        assert_eq!(SETTINGS, '\u{e57f}');
        assert_eq!(ADD_LINK, '\u{e053}');
        assert_eq!(CREATE_NEW_FOLDER, '\u{ef8c}');
        assert_eq!(DASHBOARD_CUSTOMIZE, '\u{e1b2}');
        assert_eq!(WALLPAPER, '\u{e6ca}');
        assert_eq!(CHECKLIST, '\u{e15b}');
        assert_eq!(CHECK, '\u{e156}');
        assert_eq!(ARROW_BACK, '\u{e092}');
        assert_eq!(KEYBOARD_ARROW_DOWN, '\u{e353}');
    }

    #[test]
    fn font_family_is_material_icons() {
        assert_eq!(FONT.family, iced::font::Family::Name("Material Icons"));
    }
}
