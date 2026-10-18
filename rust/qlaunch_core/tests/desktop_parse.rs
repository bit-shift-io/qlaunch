use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use qlaunch_core::model::{AppCategory, AppEntry, AppRef};
use qlaunch_core::platform::linux::{
    LinuxSource, dirs_to_scan_from, discover_applications, exec_argv, locale_from, parse_desktop,
};
use qlaunch_core::platform::{AppSource, LaunchError};

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/applications/"
    ))
    .join(relative)
}

fn parse_fixture(
    relative: &str,
    locale: Option<&str>,
) -> qlaunch_core::platform::linux::DesktopEntry {
    let path = fixture(relative);
    let id = path.file_name().unwrap().to_str().unwrap();
    parse_desktop(&fs::read_to_string(&path).unwrap(), id, locale)
}

fn ids(found: &[(String, qlaunch_core::platform::linux::DesktopEntry)]) -> Vec<&str> {
    found.iter().map(|(id, _)| id.as_str()).collect()
}

fn names(found: &[(String, qlaunch_core::platform::linux::DesktopEntry)]) -> Vec<&str> {
    found.iter().map(|(_, e)| e.name.as_str()).collect()
}

// ---- exec_argv ----

#[test]
fn exec_with_quotes_spaces_and_percent_does_not_mangle() {
    assert_eq!(
        exec_argv("/usr/bin/foo --title=\"100% done\" %U"),
        ["/usr/bin/foo", "--title=100% done"]
    );
    assert_eq!(exec_argv("foo %% %f"), ["foo", "%"]);
    assert_eq!(
        exec_argv("env FOO=bar baz arg"),
        ["env", "FOO=bar", "baz", "arg"]
    );
}

#[test]
fn exec_from_a_real_file_is_split_the_same_way() {
    let entry = parse_fixture("parse/quoted-exec.desktop", None);
    assert_eq!(
        exec_argv(entry.exec.as_deref().unwrap()),
        ["/usr/bin/foo", "--title=100% done"]
    );
}

#[test]
fn known_field_codes_are_dropped_without_eating_a_neighbouring_space() {
    for code in ["f", "F", "u", "U", "i", "c", "k"] {
        assert_eq!(
            exec_argv(&format!("foo %{code} bar")),
            ["foo", "bar"],
            "%{code}"
        );
        assert_eq!(
            exec_argv(&format!("foo %{code}")),
            ["foo"],
            "trailing %{code}"
        );
        assert_eq!(
            exec_argv(&format!("%{code} foo")),
            ["foo"],
            "leading %{code}"
        );
        assert_eq!(
            exec_argv(&format!("foo%{code}")),
            ["foo"],
            "attached %{code}"
        );
    }
    assert_eq!(exec_argv("foo %U %F"), ["foo"]);
}

#[test]
fn a_field_code_inside_an_argument_leaves_the_rest_of_it() {
    assert_eq!(exec_argv("foo --file=%f"), ["foo", "--file="]);
}

#[test]
fn a_doubled_percent_is_a_literal_percent_and_not_a_field_code() {
    assert_eq!(exec_argv("foo %%"), ["foo", "%"]);
    assert_eq!(exec_argv("foo %%f"), ["foo", "%f"]);
    assert_eq!(exec_argv("foo 100%%"), ["foo", "100%"]);
}

#[test]
fn an_unknown_or_dangling_percent_is_passed_through_literally() {
    assert_eq!(exec_argv("foo %z"), ["foo", "%z"]);
    assert_eq!(exec_argv("foo 100%"), ["foo", "100%"]);
    assert_eq!(exec_argv("foo %"), ["foo", "%"]);
    assert_eq!(exec_argv("foo 50% off"), ["foo", "50%", "off"]);
}

#[test]
fn an_argument_made_only_of_field_codes_disappears_but_an_explicit_empty_string_stays() {
    assert_eq!(exec_argv("foo %U"), ["foo"]);
    assert_eq!(exec_argv("foo %f%F bar"), ["foo", "bar"]);
    assert_eq!(exec_argv("foo \"%U\" bar"), ["foo", "bar"]);
    assert_eq!(exec_argv("foo \"\" bar"), ["foo", "", "bar"]);
    assert_eq!(exec_argv("foo \"\""), ["foo", ""]);
    assert_eq!(exec_argv("foo \"\"%U"), ["foo", ""]);
}

#[test]
fn quotes_group_words_and_may_sit_in_the_middle_of_an_argument() {
    assert_eq!(exec_argv("foo \"a b  c\" d"), ["foo", "a b  c", "d"]);
    assert_eq!(exec_argv("foo --title=\"a b\"c"), ["foo", "--title=a bc"]);
}

#[test]
fn inside_quotes_a_backslash_escapes_only_the_four_reserved_characters() {
    assert_eq!(exec_argv(r#"foo "say \"hi\"""#), ["foo", r#"say "hi""#]);
    assert_eq!(exec_argv(r#"foo "a\\b""#), ["foo", r"a\b"]);
    assert_eq!(exec_argv(r#"foo "\$HOME \`x\`""#), ["foo", "$HOME `x`"]);
    assert_eq!(exec_argv(r#"foo "C:\dir""#), ["foo", r"C:\dir"]);
}

#[test]
fn exec_is_not_a_shell_so_nothing_else_is_special() {
    assert_eq!(exec_argv("foo $HOME 'a b'"), ["foo", "$HOME", "'a", "b'"]);
    assert_eq!(exec_argv(r"foo a\ b"), ["foo", r"a\", "b"]);
}

#[test]
fn separators_are_runs_of_spaces_or_tabs_and_blank_input_is_empty() {
    assert_eq!(exec_argv("  foo \t  bar  "), ["foo", "bar"]);
    assert!(exec_argv("").is_empty());
    assert!(exec_argv("   ").is_empty());
}

#[test]
fn unbalanced_quotes_take_the_remainder_as_one_argument() {
    assert_eq!(exec_argv("foo \"bar baz"), ["foo", "bar baz"]);
    assert_eq!(exec_argv("foo --title=\"a b %U"), ["foo", "--title=a b "]);
    assert_eq!(
        exec_argv("foo \"bar\" \"baz  qux"),
        ["foo", "bar", "baz  qux"]
    );
}

// ---- parse_desktop ----

#[test]
fn every_field_is_read_from_the_desktop_entry_group_only() {
    let entry = parse_fixture("parse/full.desktop", None);
    assert_eq!(entry.id, "full.desktop");
    assert_eq!(entry.entry_type.as_deref(), Some("Application"));
    assert_eq!(entry.name, "Full Example");
    assert_eq!(entry.icon.as_deref(), Some("/usr/share/pixmaps/full.png"));
    assert_eq!(entry.exec.as_deref(), Some("/usr/bin/full --flag=x %U"));
    assert!(entry.terminal);
    assert_eq!(entry.categories, ["Utility", "TextEditor"]);
    assert_eq!(entry.only_show_in, ["GNOME", "KDE"]);
    assert_eq!(entry.not_show_in, ["XFCE"]);
    assert!(!entry.no_display);
    assert!(!entry.hidden);
}

#[test]
fn crlf_line_endings_and_a_byte_order_mark_are_tolerated() {
    let entry = parse_fixture("parse/crlf-bom.desktop", None);
    assert_eq!(entry.entry_type.as_deref(), Some("Application"));
    assert_eq!(entry.name, "Windows Editor");
    assert_eq!(entry.exec.as_deref(), Some("notepad-ish %f"));
    assert_eq!(entry.categories, ["Utility"]);
    assert!(entry.terminal);
}

#[test]
fn keys_outside_the_desktop_entry_group_are_ignored() {
    let entry = parse_fixture("parse/no-group.desktop", None);
    assert_eq!(entry.entry_type, None);
    assert_eq!(entry.name, "");
}

#[test]
fn a_missing_type_is_none_and_absent_optional_fields_are_empty() {
    let entry = parse_fixture("parse/minimal-no-type.desktop", None);
    assert_eq!(entry.entry_type, None);
    assert_eq!(entry.name, "No Type");
    assert_eq!(entry.icon, None);
    assert_eq!(entry.exec, None);
    assert!(!entry.terminal && !entry.no_display && !entry.hidden);
    assert!(entry.categories.is_empty());
    assert!(entry.only_show_in.is_empty() && entry.not_show_in.is_empty());
}

#[test]
fn a_locale_name_is_preferred_and_a_missing_name_does_not_panic() {
    assert_eq!(
        parse_fixture("parse/localized.desktop", Some("de")).name,
        "Dateien"
    );
    assert_eq!(parse_fixture("parse/no-name.desktop", Some("de")).name, "");
    assert_eq!(parse_fixture("parse/no-name.desktop", None).name, "");
}

#[test]
fn the_locale_is_matched_from_most_to_least_specific() {
    let name = |locale: Option<&str>| parse_fixture("parse/localized.desktop", locale).name;
    assert_eq!(name(None), "Files");
    assert_eq!(name(Some("de_DE.UTF-8")), "Dateien");
    assert_eq!(name(Some("de_AT.UTF-8")), "Dateien (Österreich)");
    assert_eq!(name(Some("fr_CA")), "Fichiers");
    assert_eq!(name(Some("pt_BR.UTF-8")), "Arquivos");
    assert_eq!(name(Some("sr@latin")), "Datoteke");
    assert_eq!(name(Some("sr_RS@latin")), "Datoteke");
    assert_eq!(name(Some("de_DE.UTF-8@euro")), "Dateien");
}

#[test]
fn a_locale_with_no_translation_or_no_language_falls_back_to_the_plain_name() {
    let name = |locale: Option<&str>| parse_fixture("parse/localized.desktop", locale).name;
    assert_eq!(name(Some("pt")), "Files");
    assert_eq!(name(Some("es_ES.UTF-8")), "Files");
    assert_eq!(name(Some("C")), "Files");
    assert_eq!(name(Some("POSIX")), "Files");
    assert_eq!(name(Some("")), "Files");
}

#[test]
fn the_locale_comes_from_the_first_non_empty_of_lc_all_lc_messages_lang() {
    assert_eq!(
        locale_from(Some("de"), Some("fr"), Some("es")).as_deref(),
        Some("de")
    );
    assert_eq!(
        locale_from(Some(""), Some("fr"), Some("es")).as_deref(),
        Some("fr")
    );
    assert_eq!(locale_from(None, None, Some("es")).as_deref(), Some("es"));
    assert_eq!(locale_from(Some(""), None, Some("")), None);
    assert_eq!(locale_from(None, None, None), None);
}

// ---- discover_applications ----

#[test]
fn no_display_and_hidden_and_non_application_are_skipped() {
    let found = discover_applications(&[fixture("visibility")], None, None);
    assert_eq!(ids(&found), ["visible.desktop"]);
    assert_eq!(names(&found), ["Visible"]);
}

#[test]
fn the_same_id_in_two_directories_prefers_xdg_data_home() {
    let dirs = [fixture("precedence/home"), fixture("precedence/system")];
    let found = discover_applications(&dirs, None, None);
    assert_eq!(ids(&found), ["shared.desktop", "only-system.desktop"]);
    assert_eq!(names(&found), ["From Home", "Only System"]);

    let reversed = [fixture("precedence/system"), fixture("precedence/home")];
    let found = discover_applications(&reversed, None, None);
    assert_eq!(names(&found), ["Only System", "From System"]);
}

#[test]
fn a_hidden_copy_in_an_earlier_directory_shadows_the_same_id_in_a_later_one() {
    let dirs = [fixture("shadow/home"), fixture("shadow/system")];
    let found = discover_applications(&dirs, None, None);
    assert_eq!(ids(&found), ["other.desktop"]);
}

#[test]
fn each_directory_is_read_in_file_name_order() {
    let found = discover_applications(&[fixture("ordered")], None, None);
    assert_eq!(
        ids(&found),
        ["alpha.desktop", "mid.desktop", "zeta.desktop"]
    );
}

#[test]
fn the_result_carries_the_locale_resolved_name() {
    let dir = tempfile::tempdir().unwrap();
    fs::copy(
        fixture("parse/localized.desktop"),
        dir.path().join("files.desktop"),
    )
    .unwrap();
    let found = discover_applications(&[dir.path().to_path_buf()], None, Some("de_DE.UTF-8"));
    assert_eq!(names(&found), ["Dateien"]);
}

#[test]
fn only_show_in_and_not_show_in_follow_the_current_desktop() {
    let shown = |current: Option<&str>| -> Vec<String> {
        discover_applications(&[fixture("desktops")], current, None)
            .into_iter()
            .map(|(_, e)| e.name)
            .collect()
    };
    assert_eq!(
        shown(None),
        ["Everywhere", "GNOME Only", "KDE Only", "Not KDE"]
    );
    assert_eq!(shown(Some("")), shown(None));
    assert_eq!(shown(Some("KDE")), ["Everywhere", "KDE Only"]);
    assert_eq!(
        shown(Some("GNOME")),
        ["Everywhere", "GNOME Only", "Not KDE"]
    );
    assert_eq!(
        shown(Some("X-Cinnamon:GNOME")),
        ["Everywhere", "GNOME Only", "Not KDE"]
    );
    assert_eq!(shown(Some("XFCE")), ["Everywhere", "Not KDE"]);
}

#[test]
fn a_missing_directory_or_one_that_is_a_file_is_skipped_silently() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");
    let a_file = dir.path().join("a-file");
    fs::write(&a_file, "not a directory").unwrap();

    let found = discover_applications(&[missing.clone(), a_file, fixture("ordered")], None, None);
    assert_eq!(
        ids(&found),
        ["alpha.desktop", "mid.desktop", "zeta.desktop"]
    );
    assert!(!missing.exists(), "discovery must not create anything");
}

#[cfg(unix)]
#[test]
fn an_unreadable_entry_is_skipped_without_losing_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let good = "[Desktop Entry]\nType=Application\nName=Good\n";
    fs::write(dir.path().join("good.desktop"), good).unwrap();
    fs::create_dir(dir.path().join("a-directory.desktop")).unwrap();
    std::os::unix::fs::symlink(
        dir.path().join("nowhere"),
        dir.path().join("dangling.desktop"),
    )
    .unwrap();
    fs::write(
        dir.path().join("later.desktop"),
        good.replace("Good", "Later"),
    )
    .unwrap();

    let found = discover_applications(&[dir.path().to_path_buf()], None, None);
    assert_eq!(ids(&found), ["good.desktop", "later.desktop"]);
}

#[test]
fn a_file_that_is_not_utf8_is_decoded_lossily_rather_than_dropped() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("cafe.desktop"),
        b"[Desktop Entry]\nType=Application\nName=Caf\xe9\n",
    )
    .unwrap();

    let found = discover_applications(&[dir.path().to_path_buf()], None, None);
    assert_eq!(names(&found), ["Caf\u{fffd}"]);
}

// ---- dirs_to_scan_from ----

fn scan(home_var: Option<&str>, data_dirs: Option<&str>, home: Option<&str>) -> Vec<PathBuf> {
    dirs_to_scan_from(
        home_var.map(Path::new),
        data_dirs.map(OsStr::new),
        home.map(Path::new),
    )
}

fn paths(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

const DEFAULTS: [&str; 3] = [
    "/home/u/.local/share/applications",
    "/usr/local/share/applications",
    "/usr/share/applications",
];

#[test]
fn unset_variables_get_the_xdg_defaults() {
    assert_eq!(scan(None, None, Some("/home/u")), paths(&DEFAULTS));
}

#[test]
fn empty_variables_get_the_xdg_defaults() {
    assert_eq!(scan(Some(""), Some(""), Some("/home/u")), paths(&DEFAULTS));
}

#[test]
fn explicit_values_replace_the_defaults_and_keep_their_order() {
    assert_eq!(
        scan(Some("/data"), Some("/opt/a:/opt/b"), Some("/home/u")),
        paths(&[
            "/data/applications",
            "/opt/a/applications",
            "/opt/b/applications"
        ])
    );
}

#[test]
fn xdg_data_home_alone_does_not_hide_the_system_defaults() {
    assert_eq!(
        scan(Some("/data"), None, Some("/home/u")),
        paths(&[
            "/data/applications",
            "/usr/local/share/applications",
            "/usr/share/applications"
        ])
    );
}

#[test]
fn relative_and_empty_entries_are_ignored() {
    assert_eq!(
        scan(
            Some("/data"),
            Some("rel/path:/opt/a::/opt/b:."),
            Some("/home/u")
        ),
        paths(&[
            "/data/applications",
            "/opt/a/applications",
            "/opt/b/applications"
        ])
    );
}

#[test]
fn a_relative_xdg_data_home_falls_back_to_the_home_default() {
    assert_eq!(
        scan(Some("relative"), None, Some("/home/u")),
        paths(&DEFAULTS)
    );
}

#[test]
fn a_data_dirs_value_with_nothing_usable_falls_back_to_the_defaults() {
    assert_eq!(scan(None, Some("rel:"), Some("/home/u")), paths(&DEFAULTS));
    assert_eq!(scan(None, Some(":::"), Some("/home/u")), paths(&DEFAULTS));
}

#[test]
fn without_a_usable_home_the_user_directory_is_left_out_but_the_system_ones_remain() {
    let system = paths(&["/usr/local/share/applications", "/usr/share/applications"]);
    assert_eq!(scan(None, None, None), system);
    assert_eq!(scan(None, None, Some("relative-home")), system);
}

#[test]
fn a_trailing_slash_does_not_double_up() {
    assert_eq!(
        scan(Some("/data/"), Some("/opt/a/"), None),
        paths(&["/data/applications", "/opt/a/applications"])
    );
}

// ---- LinuxSource ----

fn by_package<'a>(apps: &'a [AppEntry], package: &str) -> &'a AppEntry {
    apps.iter().find(|a| a.package == package).unwrap()
}

#[test]
fn list_apps_maps_an_entry_to_an_app_entry_with_no_activity() {
    let source = LinuxSource::new_with_dirs(vec![fixture("visibility")]);
    assert_eq!(
        source.list_apps().unwrap(),
        [AppEntry {
            package: "visible.desktop".into(),
            activity: String::new(),
            label: "Visible".into(),
            category: AppCategory::App,
        }]
    );
}

#[test]
fn categories_containing_game_map_to_the_games_group() {
    let source = LinuxSource::new_with_dirs(vec![fixture("categories")]);
    let apps = source.list_apps().unwrap();
    assert_eq!(
        by_package(&apps, "game.desktop").category,
        AppCategory::Game
    );
    assert_eq!(
        by_package(&apps, "subcategory-only.desktop").category,
        AppCategory::Game
    );
    assert_eq!(
        by_package(&apps, "mixed.desktop").category,
        AppCategory::Game
    );
}

#[test]
fn other_categories_map_to_apps_and_never_to_unknown() {
    let source = LinuxSource::new_with_dirs(vec![fixture("categories")]);
    let apps = source.list_apps().unwrap();
    assert_eq!(
        by_package(&apps, "plain.desktop").category,
        AppCategory::App
    );
    assert_eq!(by_package(&apps, "none.desktop").category, AppCategory::App);
    assert_eq!(
        by_package(&apps, "lookalike.desktop").category,
        AppCategory::App
    );
    assert!(apps.iter().all(|a| a.category != AppCategory::Unknown));
}

#[test]
fn an_entry_with_no_name_is_listed_with_an_empty_label() {
    let dir = tempfile::tempdir().unwrap();
    fs::copy(
        fixture("parse/no-name.desktop"),
        dir.path().join("nameless.desktop"),
    )
    .unwrap();
    let apps = LinuxSource::new_with_dirs(vec![dir.path().to_path_buf()])
        .list_apps()
        .unwrap();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].label, "");
}

#[test]
fn list_apps_succeeds_with_an_empty_list_when_every_directory_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let source = LinuxSource::new_with_dirs(vec![dir.path().join("nope")]);
    assert_eq!(source.list_apps(), Ok(vec![]));
}

#[test]
fn a_source_with_no_directories_has_no_icon_and_cannot_launch_anything() {
    let source = LinuxSource::new_with_dirs(vec![]);
    let app = AppRef {
        package: "visible.desktop".into(),
        activity: String::new(),
    };
    assert_eq!(source.icon_png(&app, 48), Ok(None));
    assert_eq!(source.launch(&app), Err(LaunchError::NotFound(app.clone())));
}
