use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

/// Tried in this order when two candidates are otherwise equal.
const EXTENSIONS: [&str; 3] = ["png", "svg", "xpm"];
const FALLBACK_THEME: &str = "hicolor";
const PIXMAPS: &str = "/usr/share/pixmaps";

/// The icon roots for `data_roots`: `<root>/icons` for each, in order, then
/// `/usr/share/pixmaps`, which is not under any data root but is where many
/// older applications still install their icon.
pub fn icon_dirs_from(data_roots: &[PathBuf]) -> Vec<PathBuf> {
    data_roots
        .iter()
        .map(|root| root.join("icons"))
        .chain([PathBuf::from(PIXMAPS)])
        .collect()
}

/// Whether `name` is a single plain path component, which is all a desktop
/// file may ask for by name: not empty, not `.`, and free of separators,
/// `..` and NUL.
pub(super) fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && name != "." && !name.contains(['/', '\\', '\0']) && !name.contains("..")
}

pub(super) fn is_png(path: &Path) -> bool {
    extension_rank(path) == Some(0)
}

pub(super) fn is_svg(path: &Path) -> bool {
    extension_rank(path) == Some(1)
}

/// Rasterises SVG bytes to a PNG no larger than `size_px` on its longest side,
/// preserving the aspect ratio. `None` when the bytes are not a usable SVG.
pub(super) fn rasterise_svg(bytes: &[u8], size_px: u32) -> Option<Vec<u8>> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
    let source = tree.size();
    let longest = source.width().max(source.height());
    if !longest.is_finite() || longest <= 0.0 {
        return None;
    }
    let requested = size_px.clamp(1, 1024) as f32;
    let scale = requested / longest;
    let width = (source.width() * scale).round().max(1.0) as u32;
    let height = (source.height() * scale).round().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
    let sx = if source.width() > 0.0 {
        width as f32 / source.width()
    } else {
        scale
    };
    let sy = if source.height() > 0.0 {
        height as f32 / source.height()
    } else {
        scale
    };
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().ok()
}

fn extension_rank(path: &Path) -> Option<usize> {
    let extension = path.extension()?.to_str()?;
    EXTENSIONS
        .iter()
        .position(|known| known.eq_ignore_ascii_case(extension))
}

fn is_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file())
}

/// The icon theme configured by the desktop, used when `XDG_CURRENT_THEME` is
/// unset: GTK's `settings.ini`, then Cosmic's `icon_theme`, then KDE's
/// `kdeglobals`. Only a plain theme name is accepted.
pub(crate) fn configured_theme(config: &Path) -> Option<String> {
    gtk_theme(&config.join("gtk-4.0/settings.ini"))
        .or_else(|| gtk_theme(&config.join("gtk-3.0/settings.ini")))
        .or_else(|| cosmic_theme(&config.join("cosmic/com.system76.CosmicTk/v1/icon_theme")))
        .or_else(|| kde_theme(&config.join("kdeglobals")))
}

fn gtk_theme(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    plain_theme(ini_value(&text, None, "gtk-icon-theme-name"))
}

fn kde_theme(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    plain_theme(ini_value(&text, Some("Icons"), "Theme"))
}

fn cosmic_theme(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    plain_theme(Some(unquote(text.trim())))
}

fn plain_theme(theme: Option<String>) -> Option<String> {
    theme.filter(|theme| is_plain_name(theme))
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(value)
        .to_owned()
}

/// The first `key` in `section`, or in any section when `section` is `None`.
/// Section and key names compare case-insensitively, as in a desktop settings
/// file.
fn ini_value(text: &str, section: Option<&str>, key: &str) -> Option<String> {
    let mut in_section = section.is_none();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_section = section
                .is_none_or(|wanted| line[1..line.len() - 1].trim().eq_ignore_ascii_case(wanted));
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case(key) {
            return Some(unquote(value.trim()));
        }
    }
    None
}

/// [`resolve_icon_themed`] without a named theme: `hicolor` only.
pub fn resolve_icon(icon: &str, size_px: u32, dirs: &[PathBuf]) -> Option<PathBuf> {
    resolve_icon_themed(icon, size_px, dirs, None)
}

/// Finds the file for an `Icon=` value, or `None`. Never panics, whatever the
/// value or the state of the filesystem.
///
/// - An **absolute** `icon` is the file itself: it is used when it exists, and
///   nothing else is tried.
/// - Otherwise `icon` must be a single plain path component (see
///   [`is_plain_name`]); anything else, such as `../../etc/passwd`, is `None`
///   and is never joined onto a path. A name that already ends in `png`,
///   `svg` or `xpm` is also tried as given.
///
/// Each entry of `dirs` is tried as both a themed root, laid out as
/// `<root>/<theme>/<size>/<category>/<name>.<ext>` (or the KDE order,
/// `<root>/<theme>/<category>/<size>/<name>.<ext>`), and a flat directory with
/// `<root>/<name>.<ext>`. The themes are `theme` (when given and a plain
/// name), the themes its `index.theme` inherits, and then `hicolor`. `apps` is
/// preferred over the other categories only when everything else ties.
///
/// Every candidate competes in one ranking, whichever theme or directory it
/// comes from. The winner is the first of:
///
/// 1. a `png`, then
/// 2. an `svg`, then
/// 3. an `xpm`;
///
/// and within one extension, the first of these placements:
///
/// - a sized theme directory, of the best-fitting size: the smallest at or
///   above `size_px`, or the largest available when every size is below it;
/// - `scalable`, then `symbolic`, which carry no size;
/// - the flat directory, which carries none either.
///
/// An extension class dominates everything below it, so a `png` from the
/// least-preferred location still beats an `svg` from the most-preferred one:
/// a PNG is used as it is, while an SVG is rasterised when it is loaded. An
/// `svg` is resolution-independent, so its size is no constraint, which makes
/// it better than an `xpm` at any pixel size.
///
/// Size outranks theme order within an extension and placement: with a 48
/// `png` in the named theme and a 256 `png` in `hicolor`, a request for 128 is
/// the 256 one. Only an exact tie goes to the named theme before an inherited
/// one, then to `hicolor`, then to the earlier root, and the flat directories
/// come after every theme. A size segment is `48x48` or `48` (the first number
/// counts) with any `@2x`-style suffix dropped; one that is not a number is
/// skipped.
///
/// Only regular files count: a directory, a dangling symlink or a symlink
/// loop named like an icon is skipped.
pub fn resolve_icon_themed(
    icon: &str,
    size_px: u32,
    dirs: &[PathBuf],
    theme: Option<&str>,
) -> Option<PathBuf> {
    let absolute = Path::new(icon);
    if absolute.is_absolute() {
        return is_file(absolute).then(|| absolute.to_path_buf());
    }
    if !is_plain_name(icon) {
        return None;
    }

    let file_names = file_names(icon);
    let themes = theme_chain(dirs, theme);
    best_candidate(dirs, &themes, size_px, &file_names)
}

/// The named theme, the themes it inherits, and `hicolor` last, without
/// duplicates. A cycle in `Inherits` is harmless.
fn theme_chain(dirs: &[PathBuf], theme: Option<&str>) -> Vec<String> {
    let mut chain = Vec::new();
    let mut queue: VecDeque<String> = theme
        .filter(|theme| is_plain_name(theme))
        .map(str::to_owned)
        .into_iter()
        .collect();
    while let Some(theme) = queue.pop_front() {
        if chain.contains(&theme) {
            continue;
        }
        let parents = if theme == FALLBACK_THEME {
            Vec::new()
        } else {
            inherited_themes(dirs, &theme)
        };
        chain.push(theme);
        queue.extend(parents);
    }
    if !chain.iter().any(|theme| theme == FALLBACK_THEME) {
        chain.push(FALLBACK_THEME.to_owned());
    }
    chain
}

/// The `Inherits` list of `theme`'s `index.theme`, in the first root that has
/// one. Hostile names are dropped rather than joined onto a path.
fn inherited_themes(dirs: &[PathBuf], theme: &str) -> Vec<String> {
    for root in dirs {
        let Ok(text) = fs::read_to_string(root.join(theme).join("index.theme")) else {
            continue;
        };
        let Some(value) = ini_value(&text, Some("Icon Theme"), "Inherits") else {
            continue;
        };
        return value
            .split(',')
            .map(str::trim)
            .filter(|name| is_plain_name(name))
            .map(str::to_owned)
            .collect();
    }
    Vec::new()
}

fn file_names(icon: &str) -> Vec<String> {
    let given = extension_rank(Path::new(icon)).map(|_| icon.to_owned());
    given
        .into_iter()
        .chain(EXTENSIONS.iter().map(|ext| format!("{icon}.{ext}")))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Placement {
    Sized,
    Scalable,
    Symbolic,
    Flat,
}

/// Lower is better, compared field by field: the extension decides first.
type Key = (usize, Placement, u8, u32);

/// Candidates are visited theme by theme, then root by root, then the flat
/// directories, and only a strictly better key replaces the one held, so that
/// order breaks exact ties. Within a size directory, `apps` is visited first.
fn best_candidate(
    dirs: &[PathBuf],
    themes: &[String],
    size_px: u32,
    file_names: &[String],
) -> Option<PathBuf> {
    let mut best: Option<(Key, PathBuf)> = None;
    let mut consider = |key: Key, path: PathBuf| {
        if best.as_ref().is_none_or(|(held, _)| key < *held) {
            best = Some((key, path));
        }
    };

    for theme in themes {
        for root in dirs {
            scan_theme(&root.join(theme), size_px, file_names, &mut consider);
        }
    }

    for root in dirs {
        for name in file_names {
            let path = root.join(name);
            let Some(rank) = extension_rank(&path) else {
                continue;
            };
            if is_file(&path) {
                consider((rank, Placement::Flat, 0, 0), path);
            }
        }
    }

    best.map(|(_, path)| path)
}

/// Scans one theme directory in both layouts: the standard
/// `<size>/<category>/<name>` and the KDE `<category>/<size>/<name>`.
fn scan_theme(
    theme_dir: &Path,
    size_px: u32,
    file_names: &[String],
    consider: &mut impl FnMut(Key, PathBuf),
) {
    for segment in subdirectory_names(theme_dir) {
        if let Some((placement, size)) = placement_of(&segment) {
            let size_dir = theme_dir.join(&segment);
            for category in category_names(&size_dir) {
                consider_category(
                    &size_dir.join(category),
                    placement,
                    size,
                    size_px,
                    file_names,
                    consider,
                );
            }
        } else if !segment.eq_ignore_ascii_case("cursors") {
            let category_dir = theme_dir.join(&segment);
            for size_segment in subdirectory_names(&category_dir) {
                let Some((placement, size)) = placement_of(&size_segment) else {
                    continue;
                };
                consider_category(
                    &category_dir.join(&size_segment),
                    placement,
                    size,
                    size_px,
                    file_names,
                    consider,
                );
            }
        }
    }
}

fn placement_of(segment: &str) -> Option<(Placement, Option<u32>)> {
    match segment {
        "scalable" => Some((Placement::Scalable, None)),
        "symbolic" => Some((Placement::Symbolic, None)),
        other => parse_size(other).map(|size| (Placement::Sized, Some(size))),
    }
}

fn consider_category(
    dir: &Path,
    placement: Placement,
    size: Option<u32>,
    size_px: u32,
    file_names: &[String],
    consider: &mut impl FnMut(Key, PathBuf),
) {
    for name in file_names {
        let path = dir.join(name);
        let Some(rank) = extension_rank(&path) else {
            continue;
        };
        if is_file(&path) {
            let (nearness, order) = fit(size, size_px);
            consider((rank, placement, nearness, order), path);
        }
    }
}

/// Category names of a size directory, with `apps` first and the rest
/// alphabetical, so the result does not depend on the filesystem.
fn category_names(size_dir: &Path) -> Vec<String> {
    let mut names = subdirectory_names(size_dir);
    names.sort_by_key(|name| !name.eq_ignore_ascii_case("apps"));
    names
}

/// Orders sizes: the smallest at or above the request first, then those
/// below it from the largest down. No size (`scalable`, `symbolic`) is a tie.
fn fit(size: Option<u32>, requested: u32) -> (u8, u32) {
    match size {
        None => (0, 0),
        Some(size) if size >= requested => (0, size),
        Some(size) => (1, u32::MAX - size),
    }
}

/// The UTF-8 entry names of `dir`, sorted so the result does not depend on how
/// the filesystem enumerates it. Empty when `dir` cannot be read.
fn subdirectory_names(dir: &Path) -> Vec<String> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

fn parse_size(segment: &str) -> Option<u32> {
    let without_scale = segment.split('@').next()?;
    let first = without_scale.split('x').next()?;
    if first.is_empty() || !first.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    first.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_svg_is_rasterised_at_the_requested_size_and_keeps_its_shape() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="#ff0000"/></svg>"##;
        let png = rasterise_svg(svg, 64).unwrap();
        let pixmap = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), (64, 32));
        let pixel = pixmap.pixel(0, 0).unwrap();
        assert_eq!(
            (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()),
            (255, 0, 0, 255)
        );
    }

    #[test]
    fn an_svg_that_cannot_be_parsed_is_none() {
        assert!(rasterise_svg(b"this is not an SVG", 48).is_none());
    }

    // ---- the configured icon theme ----

    fn config_with(relative: &str, contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
        dir
    }

    #[test]
    fn the_gtk_four_icon_theme_is_read() {
        let dir = config_with(
            "gtk-4.0/settings.ini",
            "[Settings]\ngtk-icon-theme-name=Breeze\n",
        );
        assert_eq!(configured_theme(dir.path()), Some("Breeze".to_owned()));
    }

    #[test]
    fn gtk_three_is_the_fallback_when_gtk_four_has_no_theme() {
        let dir = config_with("gtk-4.0/settings.ini", "[Settings]\n");
        let gtk3 = dir.path().join("gtk-3.0");
        fs::create_dir_all(&gtk3).unwrap();
        fs::write(
            gtk3.join("settings.ini"),
            "[Settings]\ngtk-icon-theme-name=Adwaita\n",
        )
        .unwrap();
        assert_eq!(configured_theme(dir.path()), Some("Adwaita".to_owned()));
    }

    #[test]
    fn the_cosmic_icon_theme_is_read_when_gtk_has_none() {
        let dir = config_with(
            "cosmic/com.system76.CosmicTk/v1/icon_theme",
            "\"breeze-dark\"\n",
        );
        assert_eq!(configured_theme(dir.path()), Some("breeze-dark".to_owned()));
    }

    #[test]
    fn the_kde_icon_theme_is_read_when_nothing_else_has_one() {
        let dir = config_with("kdeglobals", "[Icons]\nTheme=breeze\n");
        assert_eq!(configured_theme(dir.path()), Some("breeze".to_owned()));
    }

    #[test]
    fn a_hostile_configured_theme_is_ignored() {
        let dir = config_with(
            "gtk-4.0/settings.ini",
            "[Settings]\ngtk-icon-theme-name=../icons/Custom\n",
        );
        assert_eq!(configured_theme(dir.path()), None);
    }

    #[test]
    fn no_configuration_means_no_theme() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(configured_theme(dir.path()), None);
    }
}
