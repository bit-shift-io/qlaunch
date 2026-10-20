# QLaunch — Icon pipeline performance

**Design document · 2026-10-20**

Status: awaiting approval.

Extends `2026-10-18-qlaunch-flutter-rust-design.md`. It changes only how icons
are fetched, decoded and cached. It does not change app discovery, launching, the
model, or the stored schema.

---

## 1. Intent and why

Scrolling the app grid lags on the Android phone (Pixel 8, iPlay60). The
investigation found three causes:

1. **The icon cache dies with the page.** `AppCells` holds its icon futures in
   `_AppCellsState._icons` (`lib/src/ui/app_cells.dart:56`). The grid lives in a
   `PageView.builder` (`lib/src/ui/group_pager.dart:222`) with no keep-alive, so
   every page's State is disposed when the user swipes away. Returning to a page
   re-fetches and re-processes every icon.
2. **Every icon is PNG-encoded and PNG-decoded.** Android's `QLaunchBridge.iconPng`
   draws the `Drawable` into a bitmap at its **intrinsic** size (often
   192–512 px), compresses it to PNG, and sends the bytes; Dart then decodes the
   PNG and scales it down for display. Both steps are CPU and memory work per
   icon.
3. **Nothing is warm.** The first time a cell is built it shows a placeholder
   until the future resolves, so the work lands on the frames the user is
   actively scrolling.

## 2. Goals

- A given `(package, activity, size_px)` is fetched from the platform and
  **decoded at most once per session**.
- Painting an icon during a scroll is a blit of an already-decoded image, never
  a decode.
- Restarting the app reuses an **on-disk** icon cache, so a cold start does not
  re-run the platform work for icons it has seen.
- The visible page's icons are requested **before** the user scrolls to them.
- Decoded-image memory stays **bounded**.

## 3. Non-goals

- No change to what an icon looks like, its size, or `BoxFit`.
- No change to app discovery, launch, the model, settings, or the schema.
- Custom per-item icons (`Item.iconOverride`, the existing TODO in
  `app_cells.dart`) stay out of scope; they still show the platform icon.
- No search-bar or tab-strip work.

## 4. Anti-goals

- The cache must not grow without bound. Flutter's `ImageCache` already bounds
  decoded images; the disk cache needs its own cap and eviction.
- A package update must not leave a stale icon on disk forever. Today staleness
  is accepted "until the app restarts"; a persisted cache would make that
  permanent, so it must be invalidated when the app set changes.
- The fix must not move work to the UI isolate. Fetch and decode must stay off
  the frames that matter.

## 5. Design

### 5.1 What crosses the bridge: `IconImage`

Icons stop being bare `Vec<u8>` (implicitly PNG) and become a format-tagged
value:

```rust
pub enum IconFormat { Png, Rgba }        // model/icon.rs

pub struct IconImage {
    pub format: IconFormat,
    pub width: u32,   // 0 for Png
    pub height: u32,  // 0 for Png
    pub bytes: Vec<u8>,
}
```

- **Android** returns `Rgba` at **exactly** `size_px` (the drawable is scaled,
  aspect preserved and centred, when the bitmap is drawn). This removes the PNG
  encoder and the decoder, and bounds each icon to `size_px²·4` bytes.
- **Linux** keeps returning `Png` (the core has no rasteriser and an SVG cannot
  be decoded by the engine anyway); Dart decodes it and resizes it to `size_px`.

`AppSource::icon_png` is renamed `icon_image` and returns
`Result<Option<IconImage>, PlatformError>`; `api::icon_for` returns
`Option<IconImage>`.

### 5.2 Dart: one decoded image per key

A new session-scoped `IconCache` (`lib/src/state/icon_cache.dart`), owned by
`LauncherState`, maps `(package, activity, size_px)` to a cached
`Future<ImageProvider?>`. It outlives any widget, so page disposal no longer
matters and the grid and the search screen share one entry.

It turns an `IconImage` into an `ImageProvider`:

- `Png` → `ResizeImage(MemoryImage(bytes), width: size_px)`, so the engine
  decodes at display size.
- `Rgba` → a new `RawRgbaImage` provider (`lib/src/ui/raw_rgba_image.dart`) that
  feeds the bytes through `decodeImageFromPixels`.

Flutter's global `ImageCache` then caches the decoded image keyed by that
provider (bounded to 100 MB / 1000 entries by default), so a rebuild or a
re-scroll paints the cached image.

### 5.3 Rendering: `ImageProvider`, not bytes

`AppCell.icon` changes from `Uint8List?` to `ImageProvider?`, and it renders
`Image(image: icon, fit: BoxFit.contain, gaplessPlayback: true,
errorBuilder: …)`. `AppCells` stops holding its own map and asks
`LauncherState.iconFor`, which is now `Future<ImageProvider?>`.

Using an `ImageProvider` (rather than decoding inside the widget and painting a
`RawImage`) is deliberate: Flutter's image cache does the bounded decoded-image
caching for free, and tests can pass a plain `MemoryImage`.

### 5.4 Prefetch

After a snapshot is adopted, `HomeScreen` warms the current page: for each item
with an app and icons visible, it calls `LauncherState.iconFor` for the current
`size_px`, on the next idle frame. The results land in the cache; the widgets
then hit it on first build.

### 5.5 A persistent core cache (disk)

The core keeps an on-disk icon cache under the config root so a restart is warm.
Key: `(package, activity, size_px)`. Value: the `IconImage` bytes (raw `Rgba`
for Android; `Png` for Linux) plus its `format`, `width`, `height`, behind a
tiny header. It is read on a miss and written after a platform fetch. The whole
directory is cleared when the app set changes (`refresh_apps_now` saw a change),
which is the same event that already re-reconciles the model — cheap, rare, and
it satisfies the staleness anti-goal. A cap on total bytes evicts oldest-first.

No schema change: the icon cache is a sibling directory, not part of
`qlaunch.json`.

## 6. Constraints

- The bridge is generated. After changing `crate::api` types run
  `flutter_rust_bridge_codegen generate` (repo root) **and**
  `cargo build --release --manifest-path rust/qlaunch_core/Cargo.toml`, or
  `test/core_smoke_test.dart` fails on the content hash.
- The Rust core stays platform-independent. The host (Android vs Linux) chooses
  the `IconFormat`; the core only moves and stores bytes.
- Rust tests live in `rust/qlaunch_core/tests/*.rs`; Dart tests via `flutter test`.
- Ignore analyzer errors under `rust_builder/cargokit/build_tool/*` (vendored).

## 7. Review Focus

Inputs and conditions this design implies but no happy path exercises; each gets
a test in the task that owns the code:

1. **An app with no icon** — placeholder, no error, and the miss is cached so it
   is not asked for again this session.
2. **Garbage or truncated bytes for a `Png`** — the cell shows the placeholder;
   decoding never throws into the frame.
3. **A package update or install** — the disk cache is dropped, so the new icon
   is fetched rather than the stale one.
4. **`size_px` changes** (settings or device pixel ratio) — a distinct entry,
   and the old one is eventually evicted, not kept forever.
5. **Icons hidden** (`iconsVisible = false`) — no request is made at all.
6. **The disk cache cap** — writing more icons than the cap holds evicts rather
   than growing without bound, and a dropped entry is simply re-fetched.

## 8. Approved choices / left to the builder

- Cache value bytes (raw vs PNG per platform) and the small on-disk header
  format: the builder's call.
- The disk cap (a sensible default, tens of MB) and its eviction order:
  the builder's call.
- Whether the batch-fetch bridge call (one call per page instead of one per
  icon) is worth adding once prefetch and the cache are in place: deferred; not
  part of this design's required work.
