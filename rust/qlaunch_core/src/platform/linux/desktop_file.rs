use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_DATA_DIRS: [&str; 2] = ["/usr/local/share", "/usr/share"];

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DesktopEntry {
    /// The desktop-file id: the file name, e.g. `firefox.desktop`.
    pub id: String,
    /// The raw `Type` key; only `Application` is launchable here.
    pub entry_type: Option<String>,
    /// Locale-resolved; empty when the file has no `Name`.
    pub name: String,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub terminal: bool,
    pub categories: Vec<String>,
    pub no_display: bool,
    pub hidden: bool,
    pub only_show_in: Vec<String>,
    pub not_show_in: Vec<String>,
}

/// Parses the `[Desktop Entry]` group of a `.desktop` file; every other group
/// is ignored. Lines without `=` are ignored, a repeated key keeps its last
/// value, and CRLF line endings and a leading byte-order mark are tolerated.
/// `id` is the file name, which is not part of the file's contents.
///
/// `locale` is a POSIX locale such as `de_AT.UTF-8@euro`; `Name` is looked up
/// as `Name[lang_COUNTRY@MODIFIER]`, `Name[lang_COUNTRY]`, `Name[lang@MODIFIER]`
/// and `Name[lang]` in that order, falling back to the plain `Name`.
pub fn parse_desktop(text: &str, id: &str, locale: Option<&str>) -> DesktopEntry {
    let fields = desktop_entry_fields(text);
    let get = |key: &str| fields.get(key).copied();
    let non_empty = |key: &str| get(key).filter(|v| !v.is_empty()).map(str::to_owned);
    let flag = |key: &str| get(key) == Some("true");
    let list = |key: &str| -> Vec<String> {
        get(key)
            .unwrap_or("")
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()
    };

    let localized = locale
        .into_iter()
        .flat_map(locale_candidates)
        .find_map(|candidate| get(&format!("Name[{candidate}]")));

    DesktopEntry {
        id: id.to_owned(),
        entry_type: non_empty("Type"),
        name: localized.or(get("Name")).unwrap_or("").to_owned(),
        icon: non_empty("Icon"),
        exec: non_empty("Exec"),
        terminal: flag("Terminal"),
        categories: list("Categories"),
        no_display: flag("NoDisplay"),
        hidden: flag("Hidden"),
        only_show_in: list("OnlyShowIn"),
        not_show_in: list("NotShowIn"),
    }
}

fn desktop_entry_fields(text: &str) -> HashMap<&str, &str> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut fields = HashMap::new();
    let mut in_entry_group = false;
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_entry_group = &line[1..line.len() - 1] == "Desktop Entry";
        } else if in_entry_group && let Some((key, value)) = line.split_once('=') {
            fields.insert(key.trim(), value.trim());
        }
    }
    fields
}

fn locale_candidates(locale: &str) -> Vec<String> {
    let (rest, modifier) = match locale.split_once('@') {
        Some((rest, modifier)) => (rest, Some(modifier).filter(|m| !m.is_empty())),
        None => (locale, None),
    };
    let rest = rest
        .split_once('.')
        .map_or(rest, |(before_encoding, _)| before_encoding);
    let (lang, country) = match rest.split_once('_') {
        Some((lang, country)) => (lang, Some(country).filter(|c| !c.is_empty())),
        None => (rest, None),
    };
    if lang.is_empty() || lang == "C" || lang == "POSIX" {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    if let (Some(country), Some(modifier)) = (country, modifier) {
        candidates.push(format!("{lang}_{country}@{modifier}"));
    }
    if let Some(country) = country {
        candidates.push(format!("{lang}_{country}"));
    }
    if let Some(modifier) = modifier {
        candidates.push(format!("{lang}@{modifier}"));
    }
    candidates.push(lang.to_owned());
    candidates
}

/// The locale to resolve names in, from `LC_ALL`, `LC_MESSAGES` and `LANG`:
/// the first of them that is set and not empty.
pub fn locale_from(
    lc_all: Option<&str>,
    lc_messages: Option<&str>,
    lang: Option<&str>,
) -> Option<String> {
    [lc_all, lc_messages, lang]
        .into_iter()
        .flatten()
        .find(|v| !v.is_empty())
        .map(str::to_owned)
}

#[derive(Default)]
struct ArgSplitter {
    args: Vec<String>,
    current: String,
    has_arg: bool,
    in_quotes: bool,
    quote_contributed: bool,
}

impl ArgSplitter {
    fn push_char(&mut self, c: char) {
        self.current.push(c);
        self.has_arg = true;
        self.quote_contributed = true;
    }

    fn drop_field_code(&mut self) {
        self.quote_contributed = true;
    }

    fn open_quote(&mut self) {
        self.in_quotes = true;
        self.quote_contributed = false;
    }

    fn close_quote(&mut self) {
        self.in_quotes = false;
        if !self.quote_contributed {
            self.has_arg = true;
        }
    }

    fn end_arg(&mut self) {
        if self.has_arg {
            self.args.push(std::mem::take(&mut self.current));
            self.has_arg = false;
        }
    }
}

/// Splits an `Exec` value into an argument vector, ready to run directly
/// without a shell.
///
/// - Arguments are separated by runs of spaces or tabs. Double quotes group
///   words, and may start or end in the middle of an argument. Single quotes,
///   `$` and everything else outside double quotes are literal.
/// - Inside double quotes, a backslash escapes only `"`, `` ` ``, `$` and `\`;
///   before anything else it is a literal backslash.
/// - `%%` is a literal `%`. The field codes `%f %F %u %U %i %c %k` are removed
///   where they stand, leaving the text around them (and the spaces) alone.
///   Any other `%` is kept literally, as is a trailing `%`.
/// - An argument that consists of nothing but removed field codes (`%U`,
///   `"%U"`) disappears entirely. An explicitly quoted empty string (`""`) is
///   an argument and yields an empty string, because it is how a file asks for
///   one. An argument with other text in it keeps that text (`--file=%f`
///   becomes `--file=`).
/// - Unbalanced quotes do not fail: from the unmatched `"` to the end of the
///   input is taken as the rest of a single argument.
pub fn exec_argv(exec: &str) -> Vec<String> {
    let mut splitter = ArgSplitter::default();
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if splitter.in_quotes => splitter.close_quote(),
            '"' => splitter.open_quote(),
            '\\' if splitter.in_quotes && matches!(chars.peek(), Some('"' | '`' | '$' | '\\')) => {
                splitter.push_char(chars.next().expect("peeked"));
            }
            '%' => match chars.peek() {
                Some('%') => {
                    chars.next();
                    splitter.push_char('%');
                }
                Some('f' | 'F' | 'u' | 'U' | 'i' | 'c' | 'k') => {
                    chars.next();
                    splitter.drop_field_code();
                }
                _ => splitter.push_char('%'),
            },
            ' ' | '\t' if !splitter.in_quotes => splitter.end_arg(),
            _ => splitter.push_char(c),
        }
    }
    splitter.end_arg();
    splitter.args
}

/// The `applications` directories to scan, highest priority first, from the
/// values of the environment that decide them. Reads nothing itself.
///
/// Per the XDG Base Directory specification, an unset or empty
/// `xdg_data_home` means `<home>/.local/share` and an unset or empty
/// `xdg_data_dirs` means `/usr/local/share:/usr/share`. Relative and empty
/// entries are ignored, since they would resolve against whatever directory
/// the launcher was started in; when that leaves nothing usable the default
/// applies, just as if the variable were unset. A relative or missing `home`
/// leaves only the user directory out.
///
/// The defaults matter: with none applied, a system that leaves these
/// variables unset would list no applications at all.
pub fn dirs_to_scan_from(
    xdg_data_home: Option<&Path>,
    xdg_data_dirs: Option<&OsStr>,
    home: Option<&Path>,
) -> Vec<PathBuf> {
    let data_home = xdg_data_home
        .filter(|p| p.is_absolute())
        .map(Path::to_path_buf)
        .or_else(|| {
            home.filter(|p| p.is_absolute())
                .map(|h| h.join(".local").join("share"))
        });

    let mut data_dirs: Vec<PathBuf> = xdg_data_dirs
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .filter(|p| p.is_absolute())
        .collect();
    if data_dirs.is_empty() {
        data_dirs = DEFAULT_DATA_DIRS.iter().map(PathBuf::from).collect();
    }

    data_home
        .into_iter()
        .chain(data_dirs)
        .map(|dir| dir.join("applications"))
        .collect()
}

/// [`dirs_to_scan_from`] for the current process environment.
pub fn dirs_to_scan() -> Vec<PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    let data_dirs = std::env::var_os("XDG_DATA_DIRS");
    dirs_to_scan_from(
        data_home.as_deref(),
        data_dirs.as_deref(),
        dirs::home_dir().as_deref(),
    )
}

/// Reads every `.desktop` file of `dirs`, in order, and returns the
/// launchable ones as `(id, entry)` in scan order. Reads nothing from the
/// environment: `current_desktop` is the value of `XDG_CURRENT_DESKTOP`
/// (`:`-separated; absent or empty means no `OnlyShowIn`/`NotShowIn`
/// filtering) and `locale` is the one to resolve names in.
///
/// - A directory that is missing or is not a directory is skipped, as is a
///   file that cannot be read. A file that is not UTF-8 is decoded lossily.
///   Files whose names are not UTF-8 are skipped, as they have no usable id.
/// - Each directory is read in file-name order, so the result does not depend
///   on how the filesystem enumerates it.
/// - The first file with a given id wins, even when that file is then
///   filtered out (`Hidden`, `NoDisplay`, wrong `Type`, `OnlyShowIn`): that is
///   how a user's `Hidden=true` copy removes a system entry, and a later
///   directory's copy must not bring it back.
/// - Only `Type=Application` is kept, and not `NoDisplay` or `Hidden` ones.
pub fn discover_applications(
    dirs: &[PathBuf],
    current_desktop: Option<&str>,
    locale: Option<&str>,
) -> Vec<(String, DesktopEntry)> {
    let desktops: Vec<&str> = current_desktop
        .into_iter()
        .flat_map(|d| d.split(':'))
        .filter(|d| !d.is_empty())
        .collect();

    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for dir in dirs {
        for (id, path) in desktop_files(dir) {
            if seen.contains(&id) {
                continue;
            }
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            seen.insert(id.clone());
            let entry = parse_desktop(&String::from_utf8_lossy(&bytes), &id, locale);
            if is_listed(&entry, &desktops) {
                found.push((id, entry));
            }
        }
    }
    found
}

fn desktop_files(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = read
        .filter_map(Result::ok)
        .filter_map(|e| Some((e.file_name().into_string().ok()?, e.path())))
        .filter(|(name, _)| name.ends_with(".desktop"))
        .collect();
    files.sort();
    files
}

fn is_listed(entry: &DesktopEntry, desktops: &[&str]) -> bool {
    if entry.entry_type.as_deref() != Some("Application") || entry.no_display || entry.hidden {
        return false;
    }
    let named = |list: &[String]| desktops.iter().any(|d| list.iter().any(|l| l == d));
    if desktops.is_empty() {
        return true;
    }
    (entry.only_show_in.is_empty() || named(&entry.only_show_in)) && !named(&entry.not_show_in)
}
