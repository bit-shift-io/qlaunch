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

fn extension_rank(path: &Path) -> Option<usize> {
    let extension = path.extension()?.to_str()?;
    EXTENSIONS
        .iter()
        .position(|known| known.eq_ignore_ascii_case(extension))
}

fn is_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file())
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
/// `<root>/<theme>/<size>/apps/<name>.<ext>`, and a flat directory with
/// `<root>/<name>.<ext>`. The themes are `theme` (when given and a plain
/// name) and then `hicolor`; neither's `Inherits` chain is followed, and other
/// themes are not scanned. Within one theme, across all roots, the winner is
/// the first of:
///
/// 1. a `png`, of the best-fitting size;
/// 2. an `svg`, then an `xpm`, each of the best-fitting size;
/// 3. one under `scalable`;
/// 4. one under `symbolic`.
///
/// The best-fitting size is the smallest at or above `size_px`, or the largest
/// available when every size is below it. A size segment is `48x48` or `48`
/// (the first number counts) with any `@2x`-style suffix dropped; one that is
/// not a number is skipped. Ties go to the earlier root. A flat directory is
/// the last resort, after every theme, and prefers `png`, `svg`, `xpm`.
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
    let themes = theme
        .filter(|t| is_plain_name(t) && *t != FALLBACK_THEME)
        .into_iter()
        .chain([FALLBACK_THEME]);
    for theme in themes {
        if let Some(found) = best_in_theme(dirs, theme, size_px, &file_names) {
            return Some(found);
        }
    }
    best_flat(dirs, &file_names)
}

fn file_names(icon: &str) -> Vec<String> {
    let given = extension_rank(Path::new(icon)).map(|_| icon.to_owned());
    given
        .into_iter()
        .chain(EXTENSIONS.iter().map(|ext| format!("{icon}.{ext}")))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    Sized,
    Scalable,
    Symbolic,
}

/// Lower is better; compared field by field, so the class decides first.
type Key = (Class, usize, u8, u32);

fn best_in_theme(
    dirs: &[PathBuf],
    theme: &str,
    size_px: u32,
    file_names: &[String],
) -> Option<PathBuf> {
    let mut best: Option<(Key, PathBuf)> = None;
    for root in dirs {
        let theme_dir = root.join(theme);
        for segment in subdirectory_names(&theme_dir) {
            let (class, size) = match segment.as_str() {
                "scalable" => (Class::Scalable, None),
                "symbolic" => (Class::Symbolic, None),
                other => match parse_size(other) {
                    Some(size) => (Class::Sized, Some(size)),
                    None => continue,
                },
            };
            let apps = theme_dir.join(&segment).join("apps");
            for name in file_names {
                let path = apps.join(name);
                let Some(rank) = extension_rank(&path) else {
                    continue;
                };
                if !is_file(&path) {
                    continue;
                }
                let (nearness, order) = fit(size, size_px);
                let key = (class, rank, nearness, order);
                if best.as_ref().is_none_or(|(held, _)| key < *held) {
                    best = Some((key, path));
                }
            }
        }
    }
    best.map(|(_, path)| path)
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

fn best_flat(dirs: &[PathBuf], file_names: &[String]) -> Option<PathBuf> {
    let mut best: Option<(usize, PathBuf)> = None;
    for root in dirs {
        for name in file_names {
            let path = root.join(name);
            let Some(rank) = extension_rank(&path) else {
                continue;
            };
            if is_file(&path) && best.as_ref().is_none_or(|(held, _)| rank < *held) {
                best = Some((rank, path));
            }
        }
    }
    best.map(|(_, path)| path)
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
