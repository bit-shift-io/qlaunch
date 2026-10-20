# QLaunch Icon Pipeline Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the app grid scroll smoothly by fetching and decoding each icon at most once per session, caching decoded images across page disposal, returning exactly-sized raw pixels from Android, prefetching the visible page, and persisting a bounded icon cache on disk.

**Architecture:** Icons become a format-tagged `IconImage { format, width, height, bytes }` across the bridge. A session-scoped `IconCache` in `LauncherState` maps `(package, activity, size_px)` to a cached `ImageProvider`, which Flutter's bounded `ImageCache` decodes once. Android returns raw RGBA at exactly `size_px`; Linux keeps returning PNG. `AppCell` paints an `ImageProvider` instead of a `Uint8List`. The core keeps a bounded on-disk cache under the config root.

**Tech Stack:** Rust (`flutter_rust_bridge` 2.13.0, `serde`), Flutter 3.47.7 / Dart 3.13.5, Android JNI host (Kotlin), Linux host (Rust).

**Spec:** `docs/superpowers/specs/2026-10-20-qlaunch-icon-performance-design.md`

## Global Constraints

- Any change to public types/functions in `crate::api` requires (from the repo root) `flutter_rust_bridge_codegen generate` **and** `cargo build --release --manifest-path rust/qlaunch_core/Cargo.toml`, or `test/core_smoke_test.dart` fails on a content-hash mismatch.
- The Rust core stays platform-independent: the host chooses the `IconFormat`; the core only moves and stores bytes.
- No change to the stored schema (`qlaunch.json`) or `SCHEMA_VERSION`; the icon cache is a sibling directory.
- UI code never does platform work: fetching stays behind `LauncherBackend`; the icon cache is format-aware only.
- **Do not `git commit`** — the environment forbids it. End each task by recording it in `.superpowers/sdd/2026-10-20-qlaunch-icon-performance/progress.md` (with `commits: none`) instead.
- Rust: tests live in `rust/qlaunch_core/tests/*.rs`, run with `cargo test` from `rust/qlaunch_core`. Dart: `flutter test`, `flutter analyze lib test`.
- Ignore analyzer errors under `rust_builder/cargokit/build_tool/*` (vendored).
- **Measuring the win (do this once at the start and once at the end):** install a release build (`JAVA_HOME=/opt/jbr-21 flutter build apk --release && adb install -r build/app/outputs/flutter-apk/app-release.apk`), then `flutter run --profile` on the device, scroll several pages back and forth, and read worst-frame time in DevTools' Performance tab. Record both numbers in `progress.md`.

## Review Focus

Inputs and conditions the spec implies but no happy path exercises; each has a test in the task that owns the code:

1. **An app with no icon** — placeholder, no error, miss cached (Tasks 4, 7).
2. **Garbage/truncated `Png` bytes** — placeholder, decode never throws into a frame (Tasks 3, 4).
3. **A package install/update/removal** — the disk cache is dropped, so the new icon is fetched (Task 8).
4. **`size_px` changes** — distinct cache entry; old entries evicted by the cap, not kept forever (Tasks 4, 8).
5. **Icons hidden (`iconsVisible == false`)** — no request at all (Task 4, preserved).
6. **Disk cache over its cap** — evicts oldest rather than growing without bound (Task 8).

---

### Task 1: `IconImage` across the bridge (Rust, no behaviour change)

**Files:**
- Create: `rust/qlaunch_core/src/model/icon.rs`
- Modify: `rust/qlaunch_core/src/model/mod.rs` (add `mod icon; pub use icon::{IconFormat, IconImage};`)
- Modify: `rust/qlaunch_core/src/platform/mod.rs` (trait method rename + return type)
- Modify: `rust/qlaunch_core/src/platform/fake.rs`, `rust/qlaunch_core/src/platform/linux/mod.rs`, `rust/qlaunch_core/src/platform/android/mod.rs`, `rust/qlaunch_core/src/platform/android/jni_host.rs`
- Modify: `rust/qlaunch_core/src/api/api.rs:251` (`icon_for` return type)
- Test: `rust/qlaunch_core/tests/icon_image.rs` (new) and update every `icon_png(` call site in `rust/qlaunch_core/tests/*.rs`

**Interfaces:**
- Consumes: `crate::model::AppRef`, `crate::platform::PlatformError`.
- Produces:
  - `pub enum IconFormat { Png, Rgba }` (derive `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize`).
  - `pub struct IconImage { pub format: IconFormat, pub width: u32, pub height: u32, pub bytes: Vec<u8> }` (derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`).
  - `AppSource::icon_image(&self, app: &AppRef, size_px: u32) -> Result<Option<IconImage>, PlatformError>` (renamed from `icon_png`).
  - `pub async fn icon_for(package: String, activity: String, size_px: u32) -> Option<IconImage>`.
  - `IconImage::png(bytes: Vec<u8>) -> IconImage` convenience (format `Png`, sizes `0`).
- Generated Dart: `enum IconFormat { png, rgba }`, `class IconImage { final IconFormat format; final int width; final int height; final Uint8List bytes; }`.

- [ ] **Step 1: Write the failing test** in `rust/qlaunch_core/tests/icon_image.rs`

```rust
use qlaunch_core::model::{IconFormat, IconImage};

#[test]
fn png_wraps_bytes_with_no_size() {
    let img = IconImage::png(vec![1, 2, 3]);
    assert_eq!(img.format, IconFormat::Png);
    assert_eq!((img.width, img.height), (0, 0));
    assert_eq!(img.bytes, vec![1, 2, 3]);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml png_wraps_bytes -- --nocapture`
Expected: FAIL, `unresolved import ... IconImage`.

- [ ] **Step 3: Add `model/icon.rs`** with the enum, struct and `png` constructor; wire it into `model/mod.rs`.

- [ ] **Step 4: Rename the trait and its implementors.** Change `AppSource::icon_png` to `icon_image` returning `Result<Option<IconImage>, PlatformError>`. In each host, wrap the existing bytes with `IconImage::png(bytes)` (`FakeSource`, Linux `icon_image`, Android `AndroidSource`, `JniHost`). Update `api::icon_for` to return `Option<IconImage>` via the renamed call. Mechanically rename `icon_png(` → `icon_image(` and wrap expected values in every `rust/qlaunch_core/tests/*.rs` call site (e.g. `Some(vec![1,2,3])` → `Some(IconImage::png(vec![1,2,3]))`; `Ok(None)` is unchanged).

- [ ] **Step 5: Regenerate the bridge and build the host lib**

Run: `flutter_rust_bridge_codegen generate && cargo build --release --manifest-path rust/qlaunch_core/Cargo.toml`
Expected: new `IconImage`/`IconFormat` in `lib/src/rust/model.dart`; build succeeds.

- [ ] **Step 6: Run the Rust tests**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml`
Expected: PASS (all icon tests updated).

- [ ] **Step 7: Record progress** in `.superpowers/sdd/2026-10-20-qlaunch-icon-performance/progress.md`.

---

### Task 2: Dart plumbing of `IconImage` (no rendering change)

**Files:**
- Modify: `lib/src/state/launcher_backend.dart` (`iconFor` → `Future<IconImage?>`, `RustBackend` maps through)
- Modify: `lib/src/ui/app_cells.dart:125` (pass `snapshot.data?.bytes` to `AppCell`, unchanged type)
- Modify: `test/fake_backend.dart` (icons map/value type)
- Test: `test/app_grid_test.dart` icon group, `test/fake_backend.dart` helpers

**Interfaces:**
- Consumes: generated `IconImage` (Task 1).
- Produces: `LauncherBackend.iconFor(String package, String activity, int sizePx) -> Future<IconImage?>`.
- Note: `AppCell.icon` stays `Uint8List?`; `AppCells` extracts `.bytes`.

- [ ] **Step 1: Write the failing test** in `test/app_grid_test.dart` (replace the byte-valued `fake.icons` uses)

```dart
testWidgets('the backend icon bytes are what the cell draws', (tester) async {
  setSurface(tester, const Size(400, 800));
  fake.icons[(package: 'pkg.a', activity: 'Main')] =
      IconImage(format: IconFormat.png, width: 0, height: 0, bytes: _png);
  await load([_appItem('a')]);
  await pumpGrid(tester);
  expect(find.byType(Image), findsOneWidget);
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `flutter test test/app_grid_test.dart --plain-name 'the backend icon bytes are what the cell draws'`
Expected: FAIL (argument type / `fake.icons` is `Uint8List`).

- [ ] **Step 3: Implement.** `LauncherBackend.iconFor` returns `Future<IconImage?>`; `RustBackend.iconFor` forwards to the generated `rust.iconFor`. `FakeBackend.icons` becomes `Map<({String package, String activity}), IconImage>` and its `iconFor` returns `icons[...]`. In `app_cells.dart`, the `FutureBuilder<Uint8List?>` becomes `FutureBuilder<IconImage?>` and `cell(snapshot.data?.bytes)`.

- [ ] **Step 4: Update the other `fake.icons[...] = _png` sites** in `test/app_grid_test.dart` to wrap the bytes as in Step 1.

- [ ] **Step 5: Run the suite**

Run: `flutter test test/app_grid_test.dart test/app_cell_test.dart && flutter analyze lib test`
Expected: PASS (rendering is unchanged; `Image.memory` still draws the bytes).

- [ ] **Step 6: Record progress.**

---

### Task 3: `RawRgbaImage` provider

**Files:**
- Create: `lib/src/ui/raw_rgba_image.dart`
- Test: `test/raw_rgba_image_test.dart`

**Interfaces:**
- Produces: `class RawRgbaImage extends ImageProvider<RawRgbaImage>` with
  `const RawRgbaImage(Uint8List bytes, int width, int height)`; equality by
  `(identical bytes, width, height)`; `loadImage` feeds the bytes to
  `ui.decodeImageFromPixels(bytes, width, height, ui.PixelFormat.rgba8888, complete)`.

- [ ] **Step 1: Write the failing test** in `test/raw_rgba_image_test.dart`

```dart
testWidgets('a 2x2 raw image paints without error', (tester) async {
  final rgba = Uint8List.fromList(List.filled(2 * 2 * 4, 0x80));
  await tester.pumpWidget(
    Image(image: RawRgbaImage(rgba, 2, 2), width: 2, height: 2),
  );
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
});

test('equal bytes identity and size are equal keys', () {
  final rgba = Uint8List.fromList(List.filled(16, 0));
  expect(RawRgbaImage(rgba, 2, 2), RawRgbaImage(rgba, 2, 2));
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `flutter test test/raw_rgba_image_test.dart`
Expected: FAIL, `RawRgbaImage` undefined.

- [ ] **Step 3: Implement** `RawRgbaImage` (extend `ImageProvider<RawRgbaImage>`, override `obtainKey` returning `SynchronousFuture(this)`, `loadImage`, `operator ==`, `hashCode`). Use Flutter's current `loadImage(T key, ImageDecoderCallback decode)` signature.

- [ ] **Step 4: Run it to verify it passes**

Run: `flutter test test/raw_rgba_image_test.dart`
Expected: PASS.

- [ ] **Step 5: Record progress.**

---

### Task 4: Session `IconCache` and provider-based rendering

**Files:**
- Create: `lib/src/state/icon_cache.dart`
- Modify: `lib/src/state/launcher_state.dart` (own an `IconCache`; `iconFor` returns `Future<ImageProvider?>`)
- Modify: `lib/src/ui/app_cells.dart` (drop `_icons`; `FutureBuilder<ImageProvider?>`)
- Modify: `lib/src/ui/app_cell.dart` (`icon` is `ImageProvider?`; render `Image(image: icon, ...)`)
- Test: `test/icon_cache_test.dart` (new), `test/app_grid_test.dart` icons group, `test/app_cell_test.dart`

**Interfaces:**
- Consumes: `LauncherBackend.iconFor -> Future<IconImage?>` (Task 2), `RawRgbaImage` (Task 3).
- Produces:
  - `class IconCache { IconCache(LauncherBackend backend); Future<ImageProvider?> resolve(String package, String activity, int sizePx); void clear(); }` — caches by `(package, activity, sizePx)`; `Png` → `ResizeImage(MemoryImage(bytes), width: sizePx)`; `Rgba` → `RawRgbaImage(bytes, width, height)`; a failed backend call evicts the key and rethrows.
  - `LauncherState.iconFor(...) -> Future<ImageProvider?>` (catches, reports, returns `null`, never sets `error`).
  - `AppCell.icon` is `ImageProvider?`.

- [ ] **Step 1: Write the failing tests**

In `test/icon_cache_test.dart`:
```dart
test('a second resolve for the same key does not call the backend again', () async {
  fake.icons[(package: 'p', activity: 'a')] =
      IconImage(format: IconFormat.png, width: 0, height: 0, bytes: _png);
  final cache = IconCache(fake);
  final first = await cache.resolve('p', 'a', 48);
  final second = await cache.resolve('p', 'a', 48);
  expect(identical(first, second), isTrue);
  expect(fake.iconCalls, 1);
});

test('a different size is a different key', () async {
  final cache = IconCache(fake);
  await cache.resolve('p', 'a', 48);
  await cache.resolve('p', 'a', 96);
  expect(fake.iconCalls, 2);
});

test('a missing icon resolves null and is cached as a miss', () async {
  final cache = IconCache(fake);
  expect(await cache.resolve('p', 'a', 48), isNull);
  expect(await cache.resolve('p', 'a', 48), isNull);
  expect(fake.iconCalls, 1);
});
```

In `test/app_grid_test.dart` (icons group) add:
```dart
testWidgets('garbage png bytes show the placeholder, not an exception', (tester) async {
  setSurface(tester, const Size(400, 800));
  fake.icons[(package: 'pkg.a', activity: 'Main')] = IconImage(
    format: IconFormat.png,
    width: 0,
    height: 0,
    bytes: Uint8List.fromList([1, 2, 3]),
  );
  await load([_appItem('a')]);
  await pumpGrid(tester);
  await tester.pumpAndSettle();
  expect(find.byKey(AppCell.placeholderKey), findsOneWidget);
  expect(tester.takeException(), isNull);
});

testWidgets('a page disposed and rebuilt does not fetch its icons again', (tester) async {
  setSurface(tester, const Size(400, 800));
  await load([_appItem('a')]);
  await pumpGrid(tester);
  expect(fake.iconCalls, 1);
  await tester.pumpWidget(const SizedBox.shrink()); // page disposed
  await pumpGrid(tester);                            // rebuilt
  expect(fake.iconCalls, 1);                         // cache survived the widget
});
```

- [ ] **Step 2: Run them to verify they fail**

Run: `flutter test test/icon_cache_test.dart test/app_grid_test.dart`
Expected: FAIL, `IconCache` undefined / `iconCalls` is 2 after rebuild.

- [ ] **Step 3: Implement `IconCache`** (`lib/src/state/icon_cache.dart`) as specified, and give `LauncherState` a `final IconCache _icons` built from `_backend`. Change `iconFor` to `Future<ImageProvider?>`, delegating to `_icons.resolve` inside the existing try/catch (unchanged error semantics). Clear the cache from `_onEvent` when a `CoreEvent_AppsChanged()` arrives.

- [ ] **Step 4: Render providers.** In `app_cells.dart`, delete the `_icons` map and `_iconFor`, make the builder `FutureBuilder<ImageProvider?>` using `widget.state.iconFor(...)`, and pass the provider to `AppCell`. In `app_cell.dart`, change the field to `ImageProvider? icon` and render `Image(image: icon, fit: BoxFit.contain, gaplessPlayback: true, errorBuilder: (c, e, s) => placeholder)`; delete the `bytes == null` branch's use of `Uint8List` (the null check stays).

- [ ] **Step 5: Update tests.** `test/app_cell_test.dart`'s `_pumpCell` helper takes `icon` and wraps bytes in `MemoryImage` where it used to pass bytes directly. `test/app_grid_test.dart`'s `find.byType(Image)` assertions still hold.

- [ ] **Step 6: Run the suite**

Run: `flutter test && flutter analyze lib test`
Expected: PASS.

- [ ] **Step 7: Record progress.**

---

### Task 5: Micro-optimisations in `AppCell`

**Files:**
- Modify: `lib/src/ui/app_cell.dart` (`_icon` opacity branch, `RepaintBoundary`)
- Test: `test/app_cell_test.dart`

**Interfaces:** none new.

- [ ] **Step 1: Write the failing tests** in `test/app_cell_test.dart`

```dart
testWidgets('full opacity draws no Opacity layer', (tester) async {
  await _pumpCell(tester, icon: _png, settings: settingsWith(iconAlpha: 100));
  expect(find.byType(Opacity), findsNothing);
});

testWidgets('a translucent alpha still fades', (tester) async {
  await _pumpCell(tester, icon: _png, settings: settingsWith(iconAlpha: 50));
  expect(find.byType(Opacity), findsOneWidget);
});
```
(Update `_pumpCell` to accept `icon` bytes and select the provider as in Task 4.)

- [ ] **Step 2: Run them to verify they fail**

Run: `flutter test test/app_cell_test.dart`
Expected: FAIL (an `Opacity` is always present).

- [ ] **Step 3: Implement.** In `_icon`, build the `Image`/placeholder, then wrap in `Opacity` only when `opacity < 1.0`. Wrap the whole cell's returned widget in a `RepaintBoundary` in `build`.

- [ ] **Step 4: Run the suite**

Run: `flutter test test/app_cell_test.dart`
Expected: PASS.

- [ ] **Step 5: Record progress.**

---

### Task 6: Prefetch the visible page

**Files:**
- Modify: `lib/src/ui/home_screen.dart` (after a snapshot is adopted, warm the current page)
- Test: `test/home_screen_test.dart` or `test/app_grid_test.dart` (whichever owns `HomeScreen` widget tests; follow the existing file)

**Interfaces:**
- Consumes: `LauncherState.iconFor` and `LauncherState.itemsIn` / `snapshot.settings`.
- Produces: a private `_warmIcons(String groupId)` in `_HomeScreenState` that, on the next frame, computes `size_px = iconSize * devicePixelRatio` and calls `state.iconFor` for each launchable, icon-showing item in the group (ignoring results).

- [ ] **Step 1: Write the failing test**

```dart
testWidgets('opening the home screen warms its page icons', (tester) async {
  fake.icons[(package: 'pkg.a', activity: 'Main')] =
      IconImage(format: IconFormat.png, width: 0, height: 0, bytes: _png);
  await tester.pumpWidget(QLaunchApp(backendOverride: fake));
  await tester.pumpAndSettle();
  expect(fake.iconCalls, greaterThan(0));
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `flutter test test/app_grid_test.dart --plain-name 'opening the home screen warms its page icons'` (adjust the file to where `QLaunchApp` is already tested)
Expected: FAIL, `iconCalls` is 0 until a cell scrolls in (the grid does build visible cells, so this test may already pass once Task 4 lands — if so, keep it as the guard and continue).

- [ ] **Step 3: Implement `_warmIcons`**, called from the existing post-frame path that already runs when a snapshot arrives (the Home-group sync in `_onStateChanged` / after first layout). It must not fetch when `iconsVisible` is false, and must not fetch for items with no app or that are uninstalled. Use `WidgetsBinding.instance.addPostFrameCallback`.

- [ ] **Step 4: Run the suite**

Run: `flutter test && flutter analyze lib test`
Expected: PASS.

- [ ] **Step 5: Record progress.**

---

### Task 7: Android returns raw RGBA at exactly `size_px`

**Files:**
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt` (add `iconRgba`, scale to `sizePx`)
- Modify: `rust/qlaunch_core/src/platform/android/jni_host.rs` (`icon_image` calls `iconRgba`)
- Test: `rust/qlaunch_core/tests/icon_image.rs` (headless parsing of the `[I` shape), manual device check

**Interfaces:**
- Consumes: `IconImage`, `IconFormat::Rgba` (Task 1).
- Produces: Kotlin `@JvmStatic fun iconRgba(packageName: String, activity: String, sizePx: Int): IntArray?` where `out[0] = width`, `out[1] = height`, `out[2 ..]` are ARGB pixels (`Bitmap.getPixels`), scaled to fit `sizePx` with aspect preserved and centred. Rust converts each ARGB `i32` to 4 RGBA bytes and returns `IconImage { format: Rgba, width, height, bytes }`.

- [ ] **Step 1: Write the failing test** in `rust/qlaunch_core/tests/icon_image.rs`

```rust
use qlaunch_core::platform::android::{argb_pixels_to_rgba, RgbaImage};

#[test]
fn argb_int_pixels_become_rgba_bytes() {
    // 0xAABBGGRR little-endian layout as Android packs it: 0xFF00FF00 = opaque green
    let out = argb_pixels_to_rgba(&[0xFF00_FF00u32]);
    assert_eq!(out, vec![0x00, 0xFF, 0x00, 0xFF]);
}
```
(`argb_pixels_to_rgba(&[u32]) -> Vec<u8>` is a pure helper in `platform/android/mod.rs`, testable off-device. Marshal `IntArray` to `Vec<i32>` in `jni_host`, pass to the helper.)

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml argb_int_pixels -- --nocapture`
Expected: FAIL, unresolved import.

- [ ] **Step 3: Implement the helper** in `platform/android/mod.rs` and use it from `JniHost::icon_image` (call `iconRgba`, signature `(Ljava/lang/String;Ljava/lang/String;I)[I`; read `out[0]`/`out[1]`, convert `out[2..]`). Clear any pending exception and return `Ok(None)` on JNI failure, as the current `icon_png` path does.

- [ ] **Step 4: Implement `iconRgba` in Kotlin**, replacing the PNG path: draw the `Drawable` into a `Bitmap` of at most `sizePx` (preserve aspect, centre), `getPixels` into an `IntArray` with two leading size slots. Keep `drawableToBitmap` for the intrinsic case if useful; remove `iconPng` if nothing else uses it.

- [ ] **Step 5: Build and verify on device**

Run: `cargo build --release --manifest-path rust/qlaunch_core/Cargo.toml`, then `JAVA_HOME=/opt/jbr-21 flutter build apk --release`, `adb install -r build/app/outputs/flutter-apk/app-release.apk`, launch, and screenshot.
Expected: icons render; `adb logcat` shows no JNI error.

- [ ] **Step 6: Run tests**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml && flutter test`
Expected: PASS.

- [ ] **Step 7: Record progress.**

---

### Task 8: Bounded on-disk icon cache in the core

**Files:**
- Create: `rust/qlaunch_core/src/icons/store.rs` (disk cache) and `rust/qlaunch_core/src/icons/mod.rs`
- Modify: `rust/qlaunch_core/src/lib.rs` (add `mod icons;`)
- Modify: `rust/qlaunch_core/src/api/state.rs` (add `icon_cache: Option<IconDiskCache>` to `App`; build it in `install`; clear it in `refresh_apps_now` when the app set changed)
- Modify: `rust/qlaunch_core/src/api/api.rs` (`icon_for` consults the cache)
- Test: `rust/qlaunch_core/tests/icon_disk_cache.rs`

**Interfaces:**
- Consumes: `IconImage`, `IconFormat`, `AppRef` (Task 1).
- Produces:
  - `pub struct IconDiskCache { /* dir + cap */ }` with
    `IconDiskCache::new(dir: PathBuf, cap_bytes: u64) -> Self`,
    `get(&self, app: &AppRef, size_px: u32) -> Option<IconImage>`,
    `put(&self, app: &AppRef, size_px: u32, image: &IconImage)`,
    `clear(&self)`.
  - File layout in `dir`: name `{fnv64(identity):016x}.bin`; body
    `magic b"QLIC" (4) | version u8 = 1 | format u8 (0 Png, 1 Rgba) | width u32 LE | height u32 LE | key_len u32 LE | key | payload`, where `key = format!("{}\0{}\0{}", package, activity, size_px)`. A read verifies magic/version/format/key and treats any mismatch or truncation as a miss.
  - `put` deletes oldest-mtime files until the directory is at or under `cap_bytes` after the write.
- Default cap: 32 MiB. Default dir: `<config_root>/icon-cache/`.

- [ ] **Step 1: Write the failing tests** in `rust/qlaunch_core/tests/icon_disk_cache.rs`

```rust
#[test]
fn put_then_get_round_trips() { /* write Rgba image, read it back equal */ }

#[test]
fn a_corrupt_file_reads_as_a_miss() { /* truncate the file, expect None */ }

#[test]
fn a_different_key_or_size_reads_as_a_miss() { /* get with another size -> None */ }

#[test]
fn clear_removes_everything() { /* put, clear, get -> None */ }

#[test]
fn over_the_cap_evicts_oldest_first() { /* small cap, several puts, oldest gone, newest present */ }
```

Also add one test to `rust/qlaunch_core/tests/api.rs`, using its existing fake source: after `init`, warm an icon for an item (`icon_for`), confirm the `icon-cache/` directory is non-empty, then drive a package change (the fake source gains an app and `refresh_apps_now` runs) and assert the directory is empty — the cache is dropped when the app set changes.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml --test icon_disk_cache`
Expected: FAIL, `IconDiskCache` undefined.

- [ ] **Step 3: Implement `icons::store::IconDiskCache`** and the module. Use `std::hash::DefaultHasher` for the file name, `std::fs` for I/O; never panic on I/O errors (a failed write just leaves the cache cold).

- [ ] **Step 4: Wire it into `App` and `icon_for`.** `install` builds `IconDiskCache::new(config_root.join("icon-cache"), 32 * 1024 * 1024)` when a config root exists, else `None`. `api::icon_for` clones `source` and the cache handle out from under the lock, then: `cache.get(...)` → else `source.icon_image(...)` → `cache.put(...)`. `refresh_apps_now` calls `cache.clear()` on the same `changed` branch that emits `AppsChanged`.

- [ ] **Step 5: Run the tests**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml`
Expected: PASS.

- [ ] **Step 6: Record progress.**

---

## Deferred (not in this plan)

- **Batch icon fetch** (one bridge call per page): only worth it if the profile after Task 6 still shows per-icon FFI overhead. Add it as a follow-up against `AppSource` if measurements call for it.
