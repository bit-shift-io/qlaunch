use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use qlaunch_core::model::{AppRef, IconFormat, IconImage};
use qlaunch_core::platform::AppSource;
use qlaunch_core::platform::linux::{
    LinuxSource, applications_dirs_from, data_roots_from, icon_dirs_from, resolve_icon,
    resolve_icon_themed,
};

const ICONS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/icons");

fn fixture(relative: &str) -> PathBuf {
    Path::new(ICONS).join(relative)
}

fn themed() -> Vec<PathBuf> {
    vec![fixture("themed/icons")]
}

fn in_themed(relative: &str) -> Option<PathBuf> {
    Some(fixture("themed/icons").join(relative))
}

fn app(id: &str) -> AppRef {
    AppRef {
        package: id.to_owned(),
        activity: String::new(),
    }
}

// ---- resolve_icon ----

#[test]
fn an_absolute_icon_path_is_used_directly() {
    let path = fixture("decoy/secret.png");
    assert_eq!(resolve_icon(path.to_str().unwrap(), 64, &[]), Some(path));
}

#[test]
fn an_absolute_icon_path_is_used_even_outside_every_icon_root() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vendor-app.png");
    fs::write(&path, b"png").unwrap();
    assert_eq!(
        resolve_icon(path.to_str().unwrap(), 64, &themed()),
        Some(path)
    );
}

#[test]
fn an_absolute_icon_path_that_does_not_exist_is_none() {
    let missing = fixture("decoy/not-there.png");
    assert_eq!(resolve_icon(missing.to_str().unwrap(), 64, &themed()), None);
}

#[test]
fn an_absolute_icon_path_naming_a_directory_is_none() {
    let dir = fixture("decoy");
    assert_eq!(resolve_icon(dir.to_str().unwrap(), 64, &themed()), None);
}

#[test]
fn a_theme_name_resolves_through_hicolor() {
    assert_eq!(
        resolve_icon("foo", 128, &themed()),
        in_themed("hicolor/128x128/apps/foo.png")
    );
}

#[test]
fn the_smallest_size_at_or_above_the_request_is_preferred() {
    // 48 and 256 are present for `sized`.
    assert_eq!(
        resolve_icon("sized", 128, &themed()),
        in_themed("hicolor/256x256/apps/sized.png")
    );
    assert_eq!(
        resolve_icon("sized", 48, &themed()),
        in_themed("hicolor/48x48/apps/sized.png")
    );
    assert_eq!(
        resolve_icon("sized", 16, &themed()),
        in_themed("hicolor/48x48/apps/sized.png")
    );
}

#[test]
fn when_every_size_is_too_small_the_largest_wins() {
    assert_eq!(
        resolve_icon("sized", 1024, &themed()),
        in_themed("hicolor/256x256/apps/sized.png")
    );
}

#[test]
fn a_png_one_size_up_beats_an_svg_at_the_exact_size() {
    // 48x48/apps/mixed.svg and 128x128/apps/mixed.png: the PNG is usable.
    assert_eq!(
        resolve_icon("mixed", 48, &themed()),
        in_themed("hicolor/128x128/apps/mixed.png")
    );
}

#[test]
fn a_sized_svg_is_used_when_there_is_no_png_but_before_scalable() {
    // Only 48x48/apps/svgxpm.svg and 64x64/apps/svgxpm.xpm exist, request 64:
    // svg is preferred over xpm even though the xpm fits better.
    assert_eq!(
        resolve_icon("svgxpm", 64, &themed()),
        in_themed("hicolor/48x48/apps/svgxpm.svg")
    );
}

#[test]
fn a_sized_png_beats_scalable() {
    assert_eq!(
        resolve_icon("sizedvsvector", 256, &themed()),
        in_themed("hicolor/16x16/apps/sizedvsvector.png")
    );
}

#[test]
fn scalable_is_used_when_nothing_is_sized() {
    assert_eq!(
        resolve_icon("vector", 64, &themed()),
        in_themed("hicolor/scalable/apps/vector.svg")
    );
}

#[test]
fn scalable_beats_symbolic_and_symbolic_is_the_last_resort() {
    assert_eq!(
        resolve_icon("layered", 64, &themed()),
        in_themed("hicolor/scalable/apps/layered.svg")
    );
    assert_eq!(
        resolve_icon("mono", 64, &themed()),
        in_themed("hicolor/symbolic/apps/mono.svg")
    );
}

#[test]
fn a_scale_suffix_is_stripped_but_the_size_is_kept() {
    // 128x128@2x counts as 128, so it is the smallest size at or above 100.
    assert_eq!(
        resolve_icon("retina", 100, &themed()),
        in_themed("hicolor/128x128@2x/apps/retina.png")
    );
    assert_eq!(
        resolve_icon("retina", 300, &themed()),
        in_themed("hicolor/512x512/apps/retina.png")
    );
}

#[test]
fn a_segment_that_is_not_a_number_is_not_a_size() {
    assert_eq!(resolve_icon("notasize", 64, &themed()), None);
}

#[test]
fn a_name_that_already_has_an_extension_is_tried_as_given() {
    assert_eq!(
        resolve_icon("given.png", 48, &themed()),
        in_themed("hicolor/48x48/apps/given.png")
    );
}

#[test]
fn a_flat_directory_is_searched_for_the_file_directly() {
    let flat = vec![fixture("flat/pixmaps")];
    assert_eq!(
        resolve_icon("flatapp", 64, &flat),
        Some(fixture("flat/pixmaps/flatapp.png"))
    );
    assert_eq!(
        resolve_icon("flatapp.png", 64, &flat),
        Some(fixture("flat/pixmaps/flatapp.png"))
    );
    assert_eq!(
        resolve_icon("legacy", 64, &flat),
        Some(fixture("flat/pixmaps/legacy.xpm"))
    );
}

#[test]
fn themed_and_flat_roots_can_be_mixed() {
    let dirs = vec![fixture("themed/icons"), fixture("flat/pixmaps")];
    assert_eq!(
        resolve_icon("foo", 128, &dirs),
        in_themed("hicolor/128x128/apps/foo.png")
    );
    assert_eq!(
        resolve_icon("flatapp", 64, &dirs),
        Some(fixture("flat/pixmaps/flatapp.png"))
    );
}

#[test]
fn a_themed_icon_beats_a_flat_one() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("foo.png"), b"flat").unwrap();
    let dirs = vec![dir.path().to_path_buf(), fixture("themed/icons")];
    assert_eq!(
        resolve_icon("foo", 128, &dirs),
        in_themed("hicolor/128x128/apps/foo.png")
    );
}

#[test]
fn an_unresolvable_icon_returns_none_and_never_panics() {
    let mut dirs = themed();
    dirs.push(fixture("flat/pixmaps"));
    dirs.push(fixture("does/not/exist"));
    assert_eq!(
        resolve_icon("definitely-not-a-real-icon-xyz", 64, &dirs),
        None
    );
    assert_eq!(resolve_icon("foo", 64, &[]), None);
}

// ---- the named theme ----

#[test]
fn the_named_theme_is_searched_before_hicolor() {
    assert_eq!(
        resolve_icon_themed("both-themes", 32, &themed(), Some("Custom")),
        in_themed("Custom/32x32/apps/both-themes.png")
    );
    assert_eq!(
        resolve_icon_themed("both-themes", 32, &themed(), None),
        in_themed("hicolor/32x32/apps/both-themes.png")
    );
}

#[test]
fn a_png_in_hicolor_beats_an_svg_in_the_named_theme() {
    // Custom has only 128x128/apps/cross-svg.svg; hicolor has a 48x48 png.
    assert_eq!(
        resolve_icon_themed("cross-svg", 128, &themed(), Some("Custom")),
        in_themed("hicolor/48x48/apps/cross-svg.png")
    );
    assert_eq!(
        resolve_icon_themed("cross-svg", 128, &themed(), None),
        in_themed("hicolor/48x48/apps/cross-svg.png")
    );
}

#[test]
fn size_outranks_theme_order_among_pngs() {
    // Custom has a 48 png, hicolor a 256 one. For 128 the smallest size at or
    // above it is the hicolor 256, not the first theme's 48.
    let custom = Some("Custom");
    assert_eq!(
        resolve_icon_themed("cross-sizes", 128, &themed(), custom),
        in_themed("hicolor/256x256/apps/cross-sizes.png")
    );
    assert_eq!(
        resolve_icon_themed("cross-sizes", 48, &themed(), custom),
        in_themed("Custom/48x48/apps/cross-sizes.png")
    );
    assert_eq!(
        resolve_icon_themed("cross-sizes", 1024, &themed(), custom),
        in_themed("hicolor/256x256/apps/cross-sizes.png")
    );
}

#[test]
fn a_flat_png_beats_a_themed_svg() {
    // hicolor has scalable/apps/pixmapped.svg, pixmaps has pixmapped.png; a
    // sized svg at the exact size is no better.
    let dirs = vec![fixture("themed/icons"), fixture("flat/pixmaps")];
    assert_eq!(
        resolve_icon("pixmapped", 64, &dirs),
        Some(fixture("flat/pixmaps/pixmapped.png"))
    );
    assert_eq!(
        resolve_icon("sizedsvg-flatpng", 48, &dirs),
        Some(fixture("flat/pixmaps/sizedsvg-flatpng.png"))
    );
    assert_eq!(
        resolve_icon_themed("pixmapped", 64, &dirs, Some("Custom")),
        Some(fixture("flat/pixmaps/pixmapped.png"))
    );
}

#[test]
fn a_themed_png_beats_a_flat_png_whatever_the_sizes() {
    // Size-known candidates of an extension come before flat ones, even a
    // themed png far below the request.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("foo.png"), b"flat").unwrap();
    let dirs = vec![dir.path().to_path_buf(), fixture("themed/icons")];
    assert_eq!(
        resolve_icon("foo", 1024, &dirs),
        in_themed("hicolor/128x128/apps/foo.png")
    );
}

#[test]
fn a_flat_svg_is_better_than_a_flat_xpm_and_both_lose_to_a_flat_png() {
    let dir = tempfile::tempdir().unwrap();
    let flat = vec![dir.path().to_path_buf()];
    fs::write(dir.path().join("x.xpm"), b"xpm").unwrap();
    assert_eq!(resolve_icon("x", 48, &flat), Some(dir.path().join("x.xpm")));
    fs::write(dir.path().join("x.svg"), b"svg").unwrap();
    assert_eq!(resolve_icon("x", 48, &flat), Some(dir.path().join("x.svg")));
    fs::write(dir.path().join("x.png"), b"png").unwrap();
    assert_eq!(resolve_icon("x", 48, &flat), Some(dir.path().join("x.png")));
}

#[test]
fn a_scalable_png_beats_a_sized_svg() {
    let dir = tempfile::tempdir().unwrap();
    let hicolor = dir.path().join("hicolor");
    fs::create_dir_all(hicolor.join("48x48/apps")).unwrap();
    fs::create_dir_all(hicolor.join("scalable/apps")).unwrap();
    fs::write(hicolor.join("48x48/apps/odd.svg"), b"svg").unwrap();
    fs::write(hicolor.join("scalable/apps/odd.png"), b"png").unwrap();
    assert_eq!(
        resolve_icon("odd", 48, &[dir.path().to_path_buf()]),
        Some(hicolor.join("scalable/apps/odd.png"))
    );
}

#[test]
fn an_icon_missing_from_the_named_theme_falls_back_to_hicolor() {
    assert_eq!(
        resolve_icon_themed("foo", 128, &themed(), Some("Custom")),
        in_themed("hicolor/128x128/apps/foo.png")
    );
    assert_eq!(
        resolve_icon_themed("custom-only", 32, &themed(), Some("Custom")),
        in_themed("Custom/32x32/apps/custom-only.png")
    );
    assert_eq!(
        resolve_icon_themed("custom-only", 32, &themed(), None),
        None
    );
}

#[test]
fn a_theme_that_does_not_exist_is_harmless() {
    assert_eq!(
        resolve_icon_themed("foo", 128, &themed(), Some("NoSuchTheme")),
        in_themed("hicolor/128x128/apps/foo.png")
    );
}

#[test]
fn a_hostile_theme_name_is_ignored() {
    // Unguarded, both of these would reach the Custom theme's icon.
    assert_eq!(
        resolve_icon_themed("custom-only", 32, &themed(), Some("../icons/Custom")),
        None
    );
    assert_eq!(
        resolve_icon_themed("custom-only", 32, &themed(), Some("a/../Custom")),
        None
    );
}

// ---- theme inheritance ----

fn theme_with(dir: &Path, name: &str, inherits: &str) {
    let theme = dir.join(name);
    fs::create_dir_all(&theme).unwrap();
    fs::write(
        theme.join("index.theme"),
        format!("[Icon Theme]\nName={name}\nInherits={inherits}\n"),
    )
    .unwrap();
}

fn icon_at(dir: &Path, relative: &str) -> PathBuf {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"icon").unwrap();
    path
}

#[test]
fn a_named_theme_inherits_icons_from_its_parent() {
    let dir = tempfile::tempdir().unwrap();
    theme_with(dir.path(), "Child", "Parent");
    let parent = icon_at(dir.path(), "Parent/48x48/apps/inherited.png");
    assert_eq!(
        resolve_icon_themed("inherited", 48, &[dir.path().to_path_buf()], Some("Child")),
        Some(parent)
    );
}

#[test]
fn theme_inheritance_is_transitive_and_cycles_are_bounded() {
    let dir = tempfile::tempdir().unwrap();
    theme_with(dir.path(), "Child", "Middle");
    theme_with(dir.path(), "Middle", "Parent,Child");
    theme_with(dir.path(), "Parent", "Middle");
    let parent = icon_at(dir.path(), "Parent/48x48/apps/deep.png");
    assert_eq!(
        resolve_icon_themed("deep", 48, &[dir.path().to_path_buf()], Some("Child")),
        Some(parent)
    );
}

#[test]
fn a_theme_beats_its_parent_for_the_same_name_and_size() {
    let dir = tempfile::tempdir().unwrap();
    theme_with(dir.path(), "Child", "Parent");
    let child = icon_at(dir.path(), "Child/48x48/apps/shared.png");
    icon_at(dir.path(), "Parent/48x48/apps/shared.png");
    assert_eq!(
        resolve_icon_themed("shared", 48, &[dir.path().to_path_buf()], Some("Child")),
        Some(child)
    );
}

#[test]
fn hicolor_stays_the_last_fallback_after_inherited_themes() {
    let dir = tempfile::tempdir().unwrap();
    theme_with(dir.path(), "Child", "Parent");
    let hicolor = icon_at(dir.path(), "hicolor/48x48/apps/only-in-hicolor.png");
    assert_eq!(
        resolve_icon_themed(
            "only-in-hicolor",
            48,
            &[dir.path().to_path_buf()],
            Some("Child")
        ),
        Some(hicolor)
    );
}

#[test]
fn a_hostile_inherited_theme_name_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    theme_with(dir.path(), "Child", "../Parent");
    icon_at(dir.path(), "Parent/48x48/apps/escaped.png");
    assert_eq!(
        resolve_icon_themed("escaped", 48, &[dir.path().to_path_buf()], Some("Child")),
        None
    );
}

// ---- categories other than apps ----

#[test]
fn an_icon_is_found_in_a_category_other_than_apps() {
    let dir = tempfile::tempdir().unwrap();
    let printer = icon_at(dir.path(), "hicolor/48x48/devices/printer.png");
    assert_eq!(
        resolve_icon("printer", 48, &[dir.path().to_path_buf()]),
        Some(printer)
    );
}

#[test]
fn an_icon_is_found_in_a_category_without_a_size_directory() {
    let dir = tempfile::tempdir().unwrap();
    let vector = icon_at(dir.path(), "hicolor/scalable/devices/printer.svg");
    assert_eq!(
        resolve_icon("printer", 48, &[dir.path().to_path_buf()]),
        Some(vector)
    );
}

#[test]
fn the_apps_category_beats_another_category_on_an_exact_tie() {
    let dir = tempfile::tempdir().unwrap();
    let apps = icon_at(dir.path(), "hicolor/48x48/apps/shared.png");
    icon_at(dir.path(), "hicolor/48x48/devices/shared.png");
    assert_eq!(
        resolve_icon("shared", 48, &[dir.path().to_path_buf()]),
        Some(apps)
    );
}

#[test]
fn an_icon_is_found_in_a_kde_category_size_layout() {
    let dir = tempfile::tempdir().unwrap();
    let theme = icon_at(
        dir.path(),
        "Breeze/preferences/32/preferences-desktop-theme.svg",
    );
    assert_eq!(
        resolve_icon_themed(
            "preferences-desktop-theme",
            32,
            &[dir.path().to_path_buf()],
            Some("Breeze")
        ),
        Some(theme)
    );
}

#[test]
fn an_icon_is_found_in_an_inherited_kde_theme_category() {
    let dir = tempfile::tempdir().unwrap();
    theme_with(dir.path(), "Dark", "Breeze");
    let device = icon_at(dir.path(), "Breeze/devices/64/printer.svg");
    assert_eq!(
        resolve_icon_themed("printer", 64, &[dir.path().to_path_buf()], Some("Dark")),
        Some(device)
    );
}

#[test]
fn a_kde_category_does_not_hide_a_sized_standard_icon() {
    let dir = tempfile::tempdir().unwrap();
    let standard = icon_at(dir.path(), "hicolor/48x48/apps/choice.png");
    icon_at(dir.path(), "hicolor/devices/48/choice.png");
    assert_eq!(
        resolve_icon("choice", 48, &[dir.path().to_path_buf()]),
        Some(standard)
    );
}

// ---- the icon name must not escape ----

#[test]
fn a_hostile_name_never_leaves_the_icon_roots() {
    // The decoy exists, so a guard that merely failed to find it would not
    // prove anything: it is reachable through the themed shape with five `..`
    // and through the flat shape with two.
    let decoy = fixture("decoy/secret.png");
    assert!(decoy.is_file());
    let reachable = fixture("themed/icons/hicolor/48x48/apps/../../../../../decoy/secret.png");
    assert!(reachable.is_file());

    let dirs = vec![fixture("themed/icons"), fixture("themed/icons/hicolor")];
    for name in [
        "../../../etc/passwd",
        "../../../../../decoy/secret",
        "../../decoy/secret",
        "../../../../../decoy/secret.png",
        "..",
        "../",
        "a/b",
        "a\\b",
        "foo/../foo",
        "foo..bar",
        "",
        "foo\0bar",
    ] {
        assert_eq!(resolve_icon(name, 48, &dirs), None, "name {name:?}");
        assert_eq!(
            resolve_icon_themed(name, 48, &dirs, Some("Custom")),
            None,
            "name {name:?}"
        );
    }
}

#[test]
fn a_relative_path_is_not_a_theme_name() {
    assert_eq!(
        resolve_icon("hicolor/128x128/apps/foo.png", 128, &themed()),
        None
    );
}

// ---- hostile filesystems ----

#[cfg(unix)]
#[test]
fn a_symlink_loop_is_skipped_without_panicking() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let apps = dir.path().join("hicolor/48x48/apps");
    fs::create_dir_all(&apps).unwrap();
    symlink(apps.join("loop.png"), apps.join("loop.png")).unwrap();
    let dirs = vec![dir.path().to_path_buf()];
    assert_eq!(resolve_icon("loop", 48, &dirs), None);

    let flat = tempfile::tempdir().unwrap();
    symlink(flat.path().join("a.png"), flat.path().join("b.png")).unwrap();
    symlink(flat.path().join("b.png"), flat.path().join("a.png")).unwrap();
    assert_eq!(resolve_icon("a", 48, &[flat.path().to_path_buf()]), None);
    assert_eq!(
        resolve_icon(flat.path().join("a.png").to_str().unwrap(), 48, &[]),
        None
    );
}

#[test]
fn a_directory_named_like_an_icon_is_not_an_icon() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("hicolor/48x48/apps/trap.png")).unwrap();
    fs::create_dir_all(dir.path().join("flatdir.png")).unwrap();
    let dirs = vec![dir.path().to_path_buf()];
    assert_eq!(resolve_icon("trap", 48, &dirs), None);
    assert_eq!(resolve_icon("flatdir", 48, &dirs), None);
}

#[cfg(unix)]
#[test]
fn directories_with_non_utf8_names_do_not_stop_the_search() {
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(OsStr::from_bytes(b"hicolor/\xff\xfe"))).unwrap();
    let good = dir.path().join("hicolor/64x64/apps");
    fs::create_dir_all(&good).unwrap();
    fs::write(good.join("ok.png"), b"png").unwrap();
    assert_eq!(
        resolve_icon("ok", 64, &[dir.path().to_path_buf()]),
        Some(good.join("ok.png"))
    );
}

// ---- the data-roots seam ----

fn roots(data_home: Option<&str>, data_dirs: Option<&str>, home: Option<&str>) -> Vec<PathBuf> {
    data_roots_from(
        data_home.map(Path::new),
        data_dirs.map(OsStr::new),
        home.map(Path::new),
    )
}

fn paths(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

#[test]
fn data_roots_carry_the_xdg_defaults_and_filters() {
    let defaults = paths(&["/home/u/.local/share", "/usr/local/share", "/usr/share"]);
    assert_eq!(roots(None, None, Some("/home/u")), defaults);
    assert_eq!(roots(Some(""), Some(""), Some("/home/u")), defaults);
    assert_eq!(roots(Some("rel"), Some("rel:"), Some("/home/u")), defaults);
    assert_eq!(
        roots(
            Some("/data"),
            Some("rel/path:/opt/a::/opt/b:."),
            Some("/home/u")
        ),
        paths(&["/data", "/opt/a", "/opt/b"])
    );
    assert_eq!(
        roots(None, None, None),
        paths(&["/usr/local/share", "/usr/share"])
    );
}

#[test]
fn applications_and_icon_dirs_derive_from_the_same_roots() {
    let roots = paths(&["/data", "/opt/a"]);
    assert_eq!(
        applications_dirs_from(&roots),
        paths(&["/data/applications", "/opt/a/applications"])
    );
    assert_eq!(
        icon_dirs_from(&roots),
        paths(&["/data/icons", "/opt/a/icons", "/usr/share/pixmaps"])
    );
}

#[test]
fn pixmaps_are_an_icon_dir_even_when_no_data_root_is_usable() {
    assert_eq!(icon_dirs_from(&[]), paths(&["/usr/share/pixmaps"]));
}

// ---- LinuxSource::icon_image ----

fn source() -> LinuxSource {
    LinuxSource::from_parts(vec![fixture("source")], None, None, None, None)
}

#[test]
fn icon_image_reads_the_exact_bytes_of_the_resolved_file() {
    let expected = fs::read(fixture("source/icons/hicolor/48x48/apps/sourceicon.png")).unwrap();
    assert!(expected.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(
        source().icon_image(&app("withicon.desktop"), 48).unwrap(),
        Some(IconImage::png(expected))
    );
}

#[test]
fn icon_image_rasterises_an_svg_icon_to_a_png() {
    assert!(fixture("source/icons/hicolor/scalable/apps/vectoricon.svg").is_file());
    let image = source()
        .icon_image(&app("vector.desktop"), 48)
        .unwrap()
        .expect("an SVG icon is rendered instead of dropped");
    assert_eq!(image.format, IconFormat::Png);
    assert_eq!(png_size(&image.bytes), (48, 48));
}

fn png_size(bytes: &[u8]) -> (u32, u32) {
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(&bytes[12..16], b"IHDR");
    (
        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    )
}

#[test]
fn icon_image_preserves_an_svg_aspect_ratio() {
    let dir = tempfile::tempdir().unwrap();
    let applications = dir.path().join("applications");
    let icons = dir.path().join("icons/hicolor/scalable/apps");
    fs::create_dir_all(&applications).unwrap();
    fs::create_dir_all(&icons).unwrap();
    fs::write(
        applications.join("wide.desktop"),
        "[Desktop Entry]\nType=Application\nName=Wide\nExec=wide\nIcon=wide\n",
    )
    .unwrap();
    fs::write(
        icons.join("wide.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="#ff0000"/></svg>"##,
    )
    .unwrap();
    let source = LinuxSource::from_parts(vec![dir.path().to_path_buf()], None, None, None, None);

    let image = source
        .icon_image(&app("wide.desktop"), 64)
        .unwrap()
        .unwrap();
    assert_eq!(png_size(&image.bytes), (64, 32));
}

#[test]
fn icon_image_is_none_for_an_svg_that_cannot_be_parsed() {
    let dir = tempfile::tempdir().unwrap();
    let applications = dir.path().join("applications");
    let icons = dir.path().join("icons/hicolor/scalable/apps");
    fs::create_dir_all(&applications).unwrap();
    fs::create_dir_all(&icons).unwrap();
    fs::write(
        applications.join("broken.desktop"),
        "[Desktop Entry]\nType=Application\nName=Broken\nExec=broken\nIcon=broken\n",
    )
    .unwrap();
    fs::write(icons.join("broken.svg"), b"this is not an SVG").unwrap();
    let source = LinuxSource::from_parts(vec![dir.path().to_path_buf()], None, None, None, None);

    assert_eq!(source.icon_image(&app("broken.desktop"), 48).unwrap(), None);
}

#[test]
fn icon_image_is_none_for_an_icon_that_cannot_be_found() {
    assert_eq!(
        source().icon_image(&app("missing.desktop"), 48).unwrap(),
        None
    );
}

#[test]
fn icon_image_is_none_for_an_entry_with_no_icon_key() {
    assert_eq!(
        source().icon_image(&app("noicon.desktop"), 48).unwrap(),
        None
    );
}

#[test]
fn icon_image_is_none_for_an_unknown_id() {
    assert_eq!(source().icon_image(&app("nope.desktop"), 48).unwrap(), None);
}

#[test]
fn icon_image_is_none_for_a_desktop_file_that_is_not_text() {
    assert_eq!(
        source().icon_image(&app("garbage.desktop"), 48).unwrap(),
        None
    );
}

#[test]
fn icon_image_is_none_when_the_id_is_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("applications/dir.desktop")).unwrap();
    let source = LinuxSource::from_parts(vec![dir.path().to_path_buf()], None, None, None, None);
    assert_eq!(source.icon_image(&app("dir.desktop"), 48).unwrap(), None);
}

#[test]
fn icon_image_does_not_follow_a_hostile_icon_name_out_of_the_roots() {
    assert!(fixture("decoy/secret.png").is_file());
    assert_eq!(
        source().icon_image(&app("hostile.desktop"), 48).unwrap(),
        None
    );
}

#[test]
fn icon_image_does_not_follow_a_hostile_id_out_of_the_applications_dirs() {
    // Every desktop file below names a real, readable PNG by absolute path, so
    // only the guard on the id can stop the lookup: without it the file is
    // found, the icon resolves and its bytes come back.
    let icon = fixture("decoy/secret.png");
    let entry = format!(
        "[Desktop Entry]\nType=Application\nIcon={}\n",
        icon.display()
    );
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("applications/sub")).unwrap();
    fs::write(dir.path().join("applications/legit.desktop"), &entry).unwrap();
    fs::write(dir.path().join("applications/sub/inner.desktop"), &entry).unwrap();
    fs::write(dir.path().join("outside.desktop"), &entry).unwrap();
    let source = LinuxSource::from_parts(vec![dir.path().to_path_buf()], None, None, None, None);

    assert_eq!(
        source.icon_image(&app("legit.desktop"), 48).unwrap(),
        Some(IconImage::png(fs::read(&icon).unwrap())),
        "control: the same entry under a plain id resolves"
    );
    for id in [
        "../outside.desktop",
        "sub/inner.desktop",
        "./legit.desktop",
        "..",
        "",
    ] {
        assert_eq!(source.icon_image(&app(id), 48).unwrap(), None, "id {id:?}");
    }
}

#[test]
fn icon_image_honours_an_absolute_icon_path() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("applications")).unwrap();
    let icon = fixture("decoy/secret.png");
    fs::write(
        dir.path().join("applications/abs.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nIcon={}\n",
            icon.display()
        ),
    )
    .unwrap();
    let source = LinuxSource::from_parts(vec![dir.path().to_path_buf()], None, None, None, None);
    assert_eq!(
        source.icon_image(&app("abs.desktop"), 48).unwrap(),
        Some(IconImage::png(fs::read(icon).unwrap()))
    );
}

#[test]
fn icon_image_is_none_for_an_absolute_icon_path_to_a_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("applications")).unwrap();
    fs::write(
        dir.path().join("applications/gone.desktop"),
        format!("[Desktop Entry]\nIcon={}/gone.png\n", dir.path().display()),
    )
    .unwrap();
    let source = LinuxSource::from_parts(vec![dir.path().to_path_buf()], None, None, None, None);
    assert_eq!(source.icon_image(&app("gone.desktop"), 48).unwrap(), None);
}

#[test]
fn the_first_data_root_with_the_desktop_file_decides() {
    let both = LinuxSource::from_parts(
        vec![fixture("source-user"), fixture("source")],
        None,
        None,
        None,
        None,
    );
    // The user root's withicon.desktop names `useronly`, shadowing the system
    // one whose icon `sourceicon` would also resolve.
    assert_eq!(
        both.icon_image(&app("withicon.desktop"), 48).unwrap(),
        Some(IconImage::png(
            fs::read(fixture("source-user/icons/hicolor/48x48/apps/useronly.png")).unwrap()
        ))
    );
}

#[test]
fn a_desktop_file_in_a_later_data_root_is_found_past_roots_that_lack_it() {
    let source = LinuxSource::from_parts(
        vec![fixture("does/not/exist"), fixture("source")],
        None,
        None,
        None,
        None,
    );
    assert_eq!(
        source.icon_image(&app("withicon.desktop"), 48).unwrap(),
        Some(IconImage::png(
            fs::read(fixture("source/icons/hicolor/48x48/apps/sourceicon.png")).unwrap()
        ))
    );
}

#[test]
fn icon_image_uses_the_current_theme_when_given() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("applications")).unwrap();
    fs::write(
        dir.path().join("applications/t.desktop"),
        "[Desktop Entry]\nType=Application\nIcon=both-themes\n",
    )
    .unwrap();
    let hicolor = dir.path().join("icons/hicolor/32x32/apps");
    let custom = dir.path().join("icons/Custom/32x32/apps");
    fs::create_dir_all(&hicolor).unwrap();
    fs::create_dir_all(&custom).unwrap();
    fs::write(hicolor.join("both-themes.png"), b"hicolor").unwrap();
    fs::write(custom.join("both-themes.png"), b"custom").unwrap();

    let roots = vec![dir.path().to_path_buf()];
    let themed = LinuxSource::from_parts(roots.clone(), None, None, Some("Custom".into()), None);
    assert_eq!(
        themed.icon_image(&app("t.desktop"), 32).unwrap(),
        Some(IconImage::png(b"custom".to_vec()))
    );
    let plain = LinuxSource::from_parts(roots, None, None, None, None);
    assert_eq!(
        plain.icon_image(&app("t.desktop"), 32).unwrap(),
        Some(IconImage::png(b"hicolor".to_vec()))
    );
}

#[test]
fn icon_image_finds_a_pixmaps_style_icon_in_a_flat_directory_of_a_data_root() {
    // The data root's own `icons/` directory is also tried as a flat one.
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("applications")).unwrap();
    fs::create_dir_all(dir.path().join("icons")).unwrap();
    fs::write(
        dir.path().join("applications/flat.desktop"),
        "[Desktop Entry]\nType=Application\nIcon=flaticon\n",
    )
    .unwrap();
    fs::write(dir.path().join("icons/flaticon.png"), b"flat").unwrap();
    let source = LinuxSource::from_parts(vec![dir.path().to_path_buf()], None, None, None, None);
    assert_eq!(
        source.icon_image(&app("flat.desktop"), 48).unwrap(),
        Some(IconImage::png(b"flat".to_vec()))
    );
}
