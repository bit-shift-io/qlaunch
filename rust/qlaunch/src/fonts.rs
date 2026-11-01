//! Fonts for the launcher's own interface — grid, tab strip, folder and
//! shortcut labels. These follow the user's Typeface setting; the chrome does
//! not (see `theme::Text`).
//!
//! The resolved family must exist on the running platform: iced's generic
//! families point at `Open Sans` / `DejaVu Serif` / `Noto Sans Mono`, which
//! Android never loads, so Android names its serif and monospace explicitly and
//! loads the serif at start-up (`android_extra_fonts`).

use iced::Font;
use iced::font::Family;
use iced_android as platform;
use qlaunch_core::settings::Typeface;

/// The font a launcher label draws with, following the user's typeface setting.
pub(crate) fn label_font(typeface: Typeface) -> Font {
    let family = match typeface {
        Typeface::Normal | Typeface::Sans => platform::DEFAULT_FONT.family,
        Typeface::Serif => serif_family(),
        Typeface::Monospace => monospace_family(),
    };
    Font {
        family,
        ..platform::DEFAULT_FONT
    }
}

#[cfg(target_os = "android")]
fn serif_family() -> Family {
    if serif_available() {
        Family::Name("Noto Serif")
    } else {
        platform::DEFAULT_FONT.family
    }
}

#[cfg(not(target_os = "android"))]
fn serif_family() -> Family {
    Family::Serif
}

#[cfg(target_os = "android")]
fn monospace_family() -> Family {
    Family::Name("Droid Sans Mono")
}

#[cfg(not(target_os = "android"))]
fn monospace_family() -> Family {
    Family::Monospace
}

/// System fonts `iced_android` does not load, so the launcher's typeface
/// choices resolve: Android ships a serif but no monospace beyond Droid Sans
/// Mono. Empty elsewhere; the returned bytes are passed to `Application::font`.
#[cfg(target_os = "android")]
pub(crate) fn android_extra_fonts() -> Vec<Vec<u8>> {
    match std::fs::read("/system/fonts/NotoSerif-Regular.ttf") {
        Ok(bytes) => {
            SERIF_AVAILABLE.store(true, std::sync::atomic::Ordering::Relaxed);
            vec![bytes]
        }
        Err(error) => {
            log::warn!("No system serif: {error}; Serif labels use the default");
            Vec::new()
        }
    }
}

#[cfg(target_os = "android")]
fn serif_available() -> bool {
    SERIF_AVAILABLE.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(target_os = "android")]
static SERIF_AVAILABLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_and_sans_use_the_platform_default() {
        // Not the generic sans family: on Android that is "Open Sans", which is
        // never loaded, and labels would draw blank.
        let default = platform::DEFAULT_FONT.family;
        assert_eq!(label_font(Typeface::Normal).family, default);
        assert_eq!(label_font(Typeface::Sans).family, default);
    }

    #[test]
    fn labels_keep_the_platform_default_weight_and_style() {
        for typeface in [
            Typeface::Normal,
            Typeface::Sans,
            Typeface::Serif,
            Typeface::Monospace,
        ] {
            let font = label_font(typeface);
            assert_eq!(font.weight, platform::DEFAULT_FONT.weight);
            assert_eq!(font.style, platform::DEFAULT_FONT.style);
        }
    }
}
