# QLaunch — Flutter + Rust rewrite

**Design document · 2026-10-18**

Status: awaiting approval. A reviewer reads this before you do; any questions it
raises come back to you in one round.

---

## 1. Intent and why

QLaunch is an existing Android launcher (Java, package `bitshift.qlaunch`, from
2018). It works, but it is a single 32 KB `MainActivity`, a static-singleton
data layer that writes one JSON file per item, and a codebase where a quarter of
the visible features are wired to nothing.

We are rebuilding it as **Flutter UI over a Rust core**, targeting **Android**
(first-class, home-screen replacement) and **Linux Wayland** (a normal windowed
app for now, layer-shell later). The Rust core owns the application model and
all persistence; on Android it reaches into the OS through JNI for the things
only the OS can answer.

Two things make this worth doing rather than patching the Java:

1. **The data layer is the interesting part and it is platform-independent.**
   Groups, ordering, shortcuts, settings and app-list reconciliation are the
   same problem on both platforms. Written once in Rust, it is unit-testable
   without a device.
2. **The old app's bugs are structural, not incidental.** Per-item files caused
   orphaned records and a seven-day deletion delay; an `id` field was declared
   and never written; shortcuts land in the wrong group; custom icons are saved
   but never drawn. Those should be designed away, not carried forward.

**Why now:** the user wants an Android launcher they control that also moves
toward a touch-first Linux tablet/phone experience, sharing one codebase.

## 2. Goals

- **Parity with the old launcher's *working* feature set on Android**, minus the
  features the user explicitly deferred (§9).
- **A Rust core that is the single source of truth** for model and data on both
  platforms, with no business logic in Dart.
- **A Linux build that runs in a window**, discovers `.desktop` applications,
  shows them in the same group/grid UI, and launches them.
- **No network access** required for any core behaviour.
- **Testable without a phone**: the whole model, store and Linux app discovery
  run and are tested on the development machine.
- **A structure that supports Android from day one**, because Android is the
  platform that actually has to be usable.

## 3. Non-goals (v1)

- **Linux as a real shell.** No `zwlr_layer_shell_v1`, no full-screen takeover,
  no compositor integration. It's a window. Layer-shell is a later milestone and
  the design must not preclude it, but nothing is built for it now.
- **Android system integrations** the user deferred: status-bar expand/toggle,
  `SET_WALLPAPER` chooser entry, proximity lock, PIN lock, device admin /
  lock-device, Quick Settings tile, cloud backup agent.
- **Android app widgets.** The old `Widget` item type was never implemented.
- **Registering as a `CREATE_SHORTCUT` provider.** See §7.6 — the old
  implementation only ever offered "Lock Device", which was dead behind a
  commented-out device-admin receiver.
- **Receiving `INSTALL_SHORTCUT` broadcasts.** Removed from Android in API 26.
- **Nested groups** (a group inside a group). One level: root → groups → items.
- **Importing data from the Java app.** The rewrite starts fresh and builds its
  default groups on first run. The old app's groups, ordering, labels, custom
  icons and settings are not carried over. (The old app keeps its data in its
  own private storage, which a non-rooted device won't let us read anyway; an
  importer would need you to produce an export by hand. If that ever becomes
  wanted, the schema-versioned document has room for it.)
- **Live wallpapers.**

## 4. Anti-goals

These are outcomes that would make this a failure even though it runs.

- **Porting the bug list.** If the new code has the same off-by-one, the same
  orphaned records, or the same "saved but never rendered" icons, we have
  rebuilt the wrong thing.
- **Leaking platform logic upward.** Dart must not contain `if (Platform.isAndroid)`
  branching over app data, and Rust must not contain UI state.
- **A NANDroid of the Java code.** Porting `MainActivity`'s class structure into
  Dart would reproduce the tangle in a new language.
- **Unverifiable by the developer.** If neither platform can be exercised
  without flashing a device, the model was put in the wrong layer.
- **A Linux build that is a stub with no shared logic.** The Linux target has to
  use the same core, not a parallel implementation.

## 5. Constraints

| Constraint | Value |
| :--- | :--- |
| Flutter | 3.47.7 stable, Dart 3.13.5 (installed) |
| Rust | 1.98.1 (installed) |
| Bridge | `flutter_rust_bridge` v2 |
| Android | `minSdk 34`, `compileSdk`/`targetSdk` latest |
| Android targets | Pixel 8 (latest), Android 14 tablet (API 34) |
| Linux | Wayland or X11, GTK/Flutter desktop; no compositor protocol use |
| Licence | GPL-3.0 (existing `LICENSE`, retained) |
| Offline | Core behaviour must not require network |
| Name / id | Label `QLaunch`; Android applicationId `bitshift.qlaunch`; debug variant `bitshift.qlaunch.debug` |
| Config location | Linux `$XDG_CONFIG_HOME/bitshift/qlaunch/`; Android `<filesDir>/bitshift/qlaunch/` |

## 6. Decisions already made

These are the user's, not the builder's.

1. **Rust owns the model and the data end-to-end**, including on Android, where
   it reaches out to Kotlin over JNI. Not a split where Dart or Kotlin holds
   app data.
2. **Fix the old app's defects now**, before the codebase grows, rather than
   porting them faithfully.
3. **Clone the old app's functionality**, but structure it however works best
   for both platforms.
4. **Linux v1 is basic**: a window, `.desktop` discovery, icons, launching.
5. **Storage is one schema-versioned JSON document per machine**, not per-item
   files, under `$XDG_CONFIG_HOME/bitshift/qlaunch/` on Linux.
6. **`minSdk 34`**, compile/target latest.
7. **Games vs Apps** comes from Android's own `ApplicationInfo.category` /
   `FLAG_IS_GAME` — no Play Store scraping.
8. **Widgets are out of v1.**

## 7. Architecture

### 7.1 Repository layout

The Flutter project is the repository root; Rust lives beside it.

```
qlaunch/
  pubspec.yaml
  lib/                        # Dart: UI only
    main.dart
    src/
      rust/                   # flutter_rust_bridge generated bindings (checked in)
      ui/                     # screens, widgets
      state/                  # ephemeral view state (see 7.5)
  rust/
    Cargo.toml                # workspace
    qlaunch_core/             # the core crate (cdylib + rlib)
      src/
        api/                  # the FRB-facing surface
        model/                # groups, items, shortcuts, flags, ordering
        store/                # JSON persistence + migration
        search/               # query matching and ranking
        platform/
          linux/              # .desktop discovery, icon theme, spawn
          android/            # JNI: PackageManager, icons, intents, receiver
  rust_builder/               # flutter_rust_bridge native build glue
  flutter_rust_bridge.yaml
  android/                    # Flutter Android host
    app/src/main/kotlin/bitshift/qlaunch/
      MainActivity.kt         # FlutterActivity, window/theme, orientation, results
      QLaunchBridge.kt        # everything Rust calls over JNI
  linux/                      # Flutter Linux host
  test/                       # Dart tests
  docs/superpowers/specs/     # this document
```

The Android host keeps the Flutter `applicationId` and namespace `bitshift.qlaunch`.
The Kotlin package name is deliberately **not** `bitshift.qlaunch.android` — Rust
looks the class up by a fixed JNI name (§7.3) and a shorter path is less to get
wrong.

### 7.2 The Rust core

Four modules, layered so that only `platform` knows what an OS is:

- **`model`** — `Group`, `GroupRole`, `Item`, `ShortcutPayload`, `AppRef`,
  `Settings`. All mutations (add, remove, move, rename, reorder, hide, sort)
  live here as pure functions over an in-memory tree. No I/O, no OS. Fully
  unit-testable.
- **`store`** — load/save/migrate the single JSON document, atomic writes,
  icon-file management, settings. Knows a *path*, not an OS.
- **`search`** — matching and ranking over the model.
- **`platform`** — behind a trait, implemented twice:

  ```rust
  trait AppSource {
      async fn list_apps(&self) -> Vec<AppEntry>;
      async fn icon_png(&self, app: &AppRef, size_px: u32) -> Option<Vec<u8>>;
      async fn launch(&self, app: &AppRef) -> Result<(), LaunchError>;
  }
  ```

  Android implements it via JNI; Linux via XDG files and `std::process`. A
  third `fake` implementation is used by tests and by the Dart integration
  tests, so the whole app can be exercised on Linux CI with a fixed app list.

`AppEntry` is the shared shape:

```rust
pub struct AppEntry {
    pub package: String,     // Android package, or .desktop file id on Linux
    pub activity: String,    // Android activity class; "" on Linux
    pub label: String,
    pub category: AppCategory, // Game | App | Unknown
}
```

`AppRef` is the persistent reference stored in the model: package + activity,
which the platform resolves back to something launchable. The same pair is what
the old app matched on for install/uninstall reconciliation, and it was the one
thing it got right.

### 7.3 Android: the JNI boundary

The direction of the bridge matters, and it is the riskiest part of the design,
so it is pinned down here.

**Startup handshake.**

1. `MainActivity.onCreate` calls `System.loadLibrary("qlaunch_core")` *before*
   `super.onCreate` and before the Flutter engine starts. Because Kotlin loads
   the library itself, the JVM invokes Rust's `JNI_OnLoad`, where Rust captures
   the `JavaVM` into a process-global (`jni` crate + `OnceLock`). This avoids
   passing raw pointers through the FFI.
2. Rust lazily resolves the application `Context` on first use by calling the
   static `QLaunchBridge.getAppContext()` and storing a `GlobalRef`.
3. The Dart side then calls the normal `flutter_rust_bridge` init.

If step 1's ordering is broken, Rust cannot reach the OS at all — this is the
first thing to verify on a device and the reason `MainActivity` is hand-written
rather than left as the Flutter template.

**Kotlin surface Rust calls.** Deliberately small, all in `QLaunchBridge.kt`,
all returning plain data:

| Method | Purpose |
| :--- | :--- |
| `getAppContext()` | Hand the `Application` context to Rust once |
| `listLaunchActivities()` | `queryIntentActivities(MAIN + LAUNCHER)`, excluding our own package |
| `iconPng(package, activity, sizePx)` | Icon as PNG bytes (adaptive icons rendered to a bitmap) |
| `launch(package, activity)` | Start the activity, with the right flags |
| `uninstall(package)` / `appInfo(package)` | `ACTION_DELETE` / app-info screen |
| `queryShortcutProviders()` | Activities handling `ACTION_CREATE_SHORTCUT` |
| `createShortcut(package, activity)` | Start for result; the returned intent is delivered back to Rust asynchronously |
| `wallpaperPng()` | Current wallpaper as PNG bytes (§7.7) |
| `setOrientation(mode)` / `setStatusBarVisible(bool)` | Window behaviour |
| `watchPackages(bool)` | Start/stop the `PACKAGE_ADDED`/`PACKAGE_REMOVED` receiver |

**Events coming the other way.** Kotlin does not call Dart directly. It calls
into Rust, and Rust exposes them to Dart:

- Package changes → Rust re-reconciles the list and emits a model-changed
  notification. The receiver lives in Kotlin (it needs a `BroadcastReceiver`)
  but the reconciliation algorithm is Rust's, so it is the same code the Linux
  `inotify` path uses.
- Home button and back button → Rust exposes a `Stream<SystemInput>`; Dart
  subscribes and decides. `MainActivity.onNewIntent`/`onKeyDown` push into it.

**Why the buttons go through Rust rather than a platform channel:** it keeps a
single bridge, and it means Dart never needs to know that an OS event happened.
The *policy* that belongs to the model — which group is the Home group — stays
in Rust; the *view* state (is a drawer open?) stays in Dart; Dart joins them.

### 7.4 Android host details

- `MainActivity` is a `FlutterActivity`, declared with the `HOME` + `DEFAULT` +
  `LAUNCHER` intent filters, `launchMode="singleTask"`, `excludeFromRecents`,
  `stateNotNeeded`, `clearTaskOnLaunch`, and
  `configChanges="orientation|keyboardHidden|screenSize"` — the same contract
  the Java app used, because it is the contract Android expects of a home app.
- The debug variant gets a different applicationId *and* a different label, so
  it can be installed alongside a system launcher without becoming home.
- Back is swallowed at home (the old app's behaviour, and correct for a home
  app); it pops normally inside the Settings and Group Editor routes.
- The package receiver is registered only while the app is resumed, and
  reconciliation is debounced — the old app ran a 3-second polling loop and
  called `System.gc()` on pause.

### 7.5 Ephemeral vs persistent state

An explicit line, because the anti-goal is leaking one into the other.

- **Persistent (Rust):** groups, items, ordering, labels, settings, icon
  overrides, Home group identity.
- **Ephemeral (Dart):** which drawer is open, which page is showing, search
  text currently typed, dialog state, animation state.

Consequence: on resume, Dart rebuilds its view state from the model. Nothing
about the current page needs saving, and the old app's fragile "am I in focus"
flag disappears.

### 7.6 Shortcuts

Two directions, and only one is in v1.

- **Consuming (in v1).** "Add Shortcut" lists activities that handle
  `ACTION_CREATE_SHORTCUT`; picking one launches it for result; the returned
  intent (label, icon, and the inner intent) is stored as a shortcut item. This
  is the useful direction and what the old app's left drawer did.
- **Providing (not in v1).** The old app registered itself as a shortcut
  provider but only offered "Lock Device", which could never work. It will not
  be recreated.

The shortcut payload is modelled explicitly rather than as a bag of intent
fields, which is how the old app's type/data precedence bug arose:

```rust
enum ShortcutPayload {
    Android {
        action: Option<String>,
        data: Option<String>,
        mime_type: Option<String>,
        categories: Vec<String>,
        package: Option<String>,
        component: Option<String>,
        flags: i32,
        extras: Vec<Extra>,   // typed {key, kind, value}, not a "|Class=key=val|" string
    },
    Command { command: String },  // Linux: run a command
}
```

Launching reconstructs the intent from these fields with **one** code path: if
data and mime type are both present, set them together; never set one and then
clobber it with the other. The `extras` format is a list of typed entries; the
old `|String=key=value|` encoding, which required exactly three `=`-separated
tokens and crashed silently on anything else, is gone.

The old app silently ignored shortcuts whose inner intent carried
`CATEGORY_LAUNCHER` (Play Store's "add to home screen"); that filter is kept,
because such a shortcut is an app, and an app is better represented as an app.

### 7.7 Wallpaper and the background overlay

The old app let the OS wallpaper show through a translucent window and drew a
black overlay on top, with alpha from a setting. Reproducing that with Flutter
means a translucent `FlutterView`, which is a known source of compositing
problems (opaque surfaces, black flicker on resume, no reliable transparency on
some devices).

**Decision:** fetch the wallpaper as bytes (`QLaunchBridge.wallpaperPng()`) and
draw it in Flutter as the bottom layer, with the overlay on top. Same visual
result, testable, no translucent-surface fragility, and it reuses the icon byte
transport that already exists. A `WallpaperManager` change listener refreshes it.

Known limitation: a *live* wallpaper will show as a still frame. Accepted for v1.

On Linux the wallpaper concept does not exist for a windowed app; the base layer
is a solid colour and the overlay setting still applies.

## 8. Behaviour specification

This is the old app, specified, with the fixes folded in. Where a value
changes, the old behaviour is noted as **was**.

### 8.1 Screens

Four Flutter routes, replacing the old activities:

| Route | Was | Contents |
| :--- | :--- | :--- |
| **Home** | `MainActivity` | Drawers, group pager, grid |
| **Settings** | `SettingsActivity` | §8.6 |
| **Group Editor** | `GroupEditorActivity` | Add, rename, reorder, delete (ordinary groups only), and move the Apps / Games / Home roles between groups |
| **App Settings** (dialog) | `ApplicationSettingsDialog` | Label, group, category, custom icon, uninstall |

Plus dialogs for shortcut add and shortcut edit. The old `IntentActivity`
trampoline existed for `SET_WALLPAPER` / `LOCK_DEVICE` / `VIEW_GROUP` — all
deferred or dead, so it is not recreated.

### 8.2 Home screen

- **Top-level groups are pages**, swipeable left/right, **wrapping** at both
  ends (the old app's "cycle" pager). A tab strip along the top shows group
  labels; its visibility is a setting (always / landscape only / portrait only /
  never).
- **The wrapping must not be implemented the way the old app did it.** It
  inserted fake pages at each end of the adapter and every consumer had to
  remember that real group *i* was at adapter position *i+1*. One consumer
  forgot, which is exactly why "Add Shortcut" filed shortcuts into the wrong
  group. **The new design has a single pager controller that owns the mapping**
  and exposes `current_group_id()`; no other code sees an adapter position. This
  is a correctness requirement, not a preference.
- **Grid.** Each page is a grid of its group's visible items. Columns come from
  the settings; portrait and landscape are configured independently (**was:**
  one "width" setting plus a "height" setting that was secretly reused as the
  landscape column count).
- **Three cell layouts**, from the setting: *tiles* (icon above label), *list*
  (icon left of label), *centred* (**was:** broken — label and icon both
  centred in the same parent, so the label drew on top of the icon; fixed by
  giving the centred layout a real vertical stack).
- **Tap** launches. **Long-press** opens App Settings (or Shortcut Edit).
- **Home button** closes an open drawer; otherwise goes to the group holding the
  `Home` role, or the first page if no group holds it. (**was:** it could also
  toggle the notification shade — deferred.)
- **Back button** at home does nothing; inside Settings/Group Editor it pops.
- **Status bar:** shown or hidden per setting. Content insets are respected
  rather than the old `FLAG_LAYOUT_NO_LIMITS` plus manual
  `getStatusBarHeight()` padding.

### 8.3 Drawers

- **Left:** Add Shortcut · Group Editor · Launcher Settings.
  (**was:** four items including "Select Wallpaper", which existed to set a
  plain black wallpaper — deferred.) The old entries all used the app icon as
  their icon, which looked broken; real icons here.
- **Right:** search field plus a grid of *all* installed apps, including apps in
  the Hidden group. Tap launches, long-press opens App Settings. Search is
  filtered by the Rust core, not by an Android `Filter`.

### 8.4 Data model

```rust
struct Group {
    id: Uuid,
    label: String,
    parent_id: Option<Uuid>,   // None = root's child = a page
    role: Option<GroupRole>,   // None for ordinary user groups
    order: i32,                // explicit; see ordering below
}

struct Item {
    id: Uuid,
    group_id: Uuid,
    label: String,
    label_overridden: bool,
    app: Option<AppRef>,           // an installed application
    shortcut: Option<ShortcutPayload>,
    uninstalled: bool,             // set when the app disappears from the OS
    order: i32,
    icon_override: Option<String>, // filename under icons/, not a bool + implicit path
}

enum GroupRole { Root, Apps, Games, Home, Hidden }
```

**Roles, not flags.** The old app encoded six special-group meanings as bit
flags (`APP_GROUP`, `GAME_GROUP`, `HOME_GROUP`, `ROOT_GROUP`, `HIDDEN`,
`VISIBLE`) on a shared `GroupItem` type, which is why visibility and purpose got
confused. They are now an enum on `Group` only.

**Roles are reassignable**, which is what the old app's Group Editor checkbox
did and what we are keeping. It follows that the invariant is "exactly one
holder", not "these particular four groups", and the rules below are what make
reassignment safe:

- Exactly one `Root`, exactly one `Hidden`, exactly one `Apps`, exactly one
  `Games`. These can move between groups but never be unset — so "where do new
  apps go?" always has an answer.
- At most one `Home`; it *can* be unset, in which case the Home button goes to
  the first page.
- `set_group_role(id, role)` moves the role: the previous holder loses it. It
  cannot be used to remove `Root`/`Hidden`/`Apps`/`Games`, only to move them.
- A group holding any role cannot be deleted. Deleting an ordinary group moves
  its items to the `Apps` group — the old app's behaviour, and the only
  non-arbitrary choice.
- **Visibility is group membership, nothing else.** The old item-level
  `VISIBLE`/`HIDDEN` flags are gone; there is no `set_item_flag`. An item is
  hidden iff its group is the `Hidden` group; a group is a page iff it is a
  child of `Root` and does not hold the `Hidden` role. The App Settings group
  picker can move an item to `Hidden`, which is the whole hiding mechanism.
- **`AppRef` is unique across all items.** One item per (package, activity) in
  the whole model. This is what makes reconciliation a clean set-diff and makes
  "return to its previous group after reinstall" well-defined; the store
  de-duplicates on load and reports it rather than silently keeping both.
- `Group.category` from the old model is deleted — it held a scraped Play Store
  genre that §8.5 no longer needs.

**Ordering is explicit.** The old app sorted items alphabetically *and* sorted
top-level groups by an `override_position` long, and the two schemes collided
(new items jumped around; group reordering mutated positions). Now every list
has an integer `order`; "sort alphabetically" is an *action* that rewrites the
orders once. **was:** implicit alphabetical-inside / positioned-outside mix.

**One level of groups.** Root has child groups; groups have child items. The old
model was fully recursive but nothing exercised it.

**Default groups on first run:** `Apps`, `Home`, `Games`, `Hidden`, in that
order, carrying the roles of the same names. New apps are filed by category
(§8.5).

**The Hidden group now works.** **was:** created by the default-group routine,
never rendered, never selectable — dead. Now it is excluded from the pager but
is a valid destination in App Settings' group picker, which is what "hiding" an
app should mean.

### 8.5 App discovery and reconciliation

- **Discovery:** `queryIntentActivities(MAIN + LAUNCHER)`, minus our own package.
  **was:** the old code had no self-filter despite a changelog entry claiming
  otherwise, so QLaunch could list itself. That filter is now explicit and
  tested.
- **Category:** `ApplicationInfo.category == CATEGORY_GAME` or `FLAG_IS_GAME`
  routes the app to the group holding the `Games` role; everything else goes to
  the group holding the `Apps` role. (**was:** an async jsoup scrape of
  `play.google.com/store/apps/details` that silently fell back to "" when
  offline, so new apps landed in Apps and then jumped to Games later.)
- **Reconciliation** on package add/remove: diff the live list against the
  model by (package, activity). Appearances are added; disappearances are marked
  `UNINSTALLED`. An app that reappears and matches an `UNINSTALLED` record
  returns to **its previous group**, which was a genuinely good behaviour worth
  keeping.
- **Removal is immediate.** **was:** JSON files younger than 7 days were never
  deleted, so uninstalled items lingered.
- **No polling.** **was:** a 3-second handler loop plus `System.gc()` on pause.

### 8.6 Settings

All settings live in the JSON document, owned by Rust (**was:**
`SharedPreferences`, with the UI on a deprecated `PreferenceActivity`). Grouped
as before: Display, Window, Icon, Text, General, Advanced.

| Setting | Type | Default | Range / values | Effect |
| :--- | :--- | :--- | :--- | :--- |
| `grid_columns_portrait` | int | 4 | 1–12 | Grid columns, portrait |
| `grid_columns_landscape` | int | 6 | 1–12 | Grid columns, landscape (**was:** conflated with "grid height") |
| `layout` | enum | `tiles` | `tiles` \| `list` \| `centred` | Cell layout |
| `tab_visibility` | enum | `always` | `always` \| `landscape_only` \| `portrait_only` \| `never` | Tab strip visibility |
| `wallpaper_alpha` | int | 0 | 0–100 | Overlay opacity; 0 = pure wallpaper, 100 = black (**was:** an undocumented `×2.55` conversion and a code default of 30 that disagreed with the XML default of 0) |
| `status_bar_visible` | bool | true | — | Show the OS status bar |
| `icons_visible` | bool | true | — | Show icons |
| `icon_size` | int (dp) | 40 | 0–100 | Icon size |
| `icon_alpha` | int | 100 | 0–100 | Icon opacity |
| `labels_visible` | bool | true | — | Show labels |
| `label_size` | int (sp) | 14 | 0–100 | Label size |
| `label_typeface` | enum | `normal` | `normal` \| `sans` \| `serif` \| `monospace` | Typeface |
| `label_colour` | enum | `white` | `white` \| `black` \| `grey` | Colour |
| `label_shadow` | bool | true | — | Drop shadow |
| `orientation` | enum | `auto` | `auto` \| `landscape` \| `portrait` | Requested orientation (Android) |
| `restart_launcher` | action | — | — | Restart the app |
| `factory_reset` | action | — | — | Wipe settings and model, with confirmation |

Dropped relative to the old app: `proximity_lock`, `pin_lock` (which was a
"Disable Long Press" checkbox that never asked for a PIN), `toggle_status_bar`.

Changed: value clamping writes the *clamped* value back rather than silently
writing the default over the user's entry.

### 8.7 Persistence

One document, `<config_root>/bitshift/qlaunch/qlaunch.json`, where config root
is `$XDG_CONFIG_HOME` (or `~/.config`) on Linux and `<filesDir>` on Android.
**Rust resolves this path in its platform layer** (`dirs` on Linux, a JNI call
to `getFilesDir` on Android); Dart never computes a platform path, which is the
§4 anti-goal. `init` takes an optional override, used only by tests.
Written atomically: serialise → temp file in the same directory → `fsync` →
`rename`. A corrupt or unreadable document is moved aside rather than
overwritten.

```json
{
  "schema": 1,
  "settings": { "grid_columns_portrait": 4, "...": "..." },
  "root_group_id": "…",
  "groups": [
    { "id": "…", "label": "Apps", "parent_id": "…", "role": "apps", "order": 0 }
  ],
  "items": [
    { "id": "…", "group_id": "…", "label": "Firefox", "label_overridden": false,
      "kind": "app", "app": { "package": "…", "activity": "…" },
      "uninstalled": false, "order": 0, "icon_override": null }
  ]
}
```

- **Real JSON types.** **was:** every value written as a string and re-read
  through `org.json`'s coercions.
- **The id is written.** **was:** an `id` key was declared and never serialised;
  identity lived only in the filename, so the field was meaningless.
- **`schema` is present and checked** on load, with a migration step, so a
  future format change is not a data-loss event. **was:** no version field.
- **Custom icons** are files: `<config_root>/bitshift/qlaunch/icons/<uuid>.png`,
  referenced by name from `icon_override`. They are actually rendered —
  **was:** saved to disk and never drawn, because the render path always called
  `PackageManager.getApplicationIcon`.

### 8.8 Linux platform layer

- **Discovery:** `.desktop` files from `$XDG_DATA_HOME/applications` and each
  `$XDG_DATA_DIRS/applications`, per the XDG Desktop Entry Specification.
  Skip `Hidden=true`, `NoDisplay=true`, non-`Application` types, and entries
  excluded by `OnlyShowIn`/`NotShowIn` for the current desktop. Deduplicate by
  desktop-file id, with `$XDG_DATA_HOME` winning.
- **Labels:** `Name`, preferring a locale match, falling back to `Name`.
- **Category:** the `Categories=` key containing `Game` maps to
  `AppCategory::Game`, so discovered games land in the Games group on first run
  the same way they do on Android (§8.5). Everything else is `App`. Without
  this, Games would be permanently empty on Linux.
- **Icons:** `Icon=` is either an absolute path or a theme name. Theme names are
  resolved against the icon theme directories with a size preference and a
  `hicolor` fallback. PNG and SVG both become bytes; SVG needs a rasteriser —
  if that proves awkward, an SVG icon renders as a placeholder while PNGs work.
- **Launching:** parse `Exec` with shell-style quoting, strip field codes
  (`%f %F %u %U %i %c %k`), spawn detached via `std::process::Command`. If
  `Terminal=true`, use `$TERMINAL` or `x-terminal-emulator`; if neither exists,
  refuse with a clear error rather than silently doing nothing.
- **Refresh:** watch the applications directories (`inotify`) and re-reconcile,
  reusing the Android reconciliation algorithm.
- **Shortcuts:** a Linux shortcut is `ShortcutPayload::Command`. A simple
  name-plus-command form. If this turns out to be more trouble than it is worth,
  the model already has room and the UI can ship without it.

### 8.9 The bridge surface

Rust exposes roughly this to Dart, all `async`, all data-in/data-out:

```rust
// lifecycle / model
async fn init(config_root_override: Option<String>) -> LaunchState; // None: Rust resolves it (§8.7)
async fn refresh_apps() -> ModelSnapshot;
async fn snapshot() -> ModelSnapshot;          // groups, items, settings
// mutations
async fn add_group(label: String) -> ModelSnapshot;
async fn rename_group(id: String, label: String) -> ModelSnapshot;
async fn move_group(id: String, new_order: i32) -> ModelSnapshot;
async fn set_group_role(id: String, role: Option<GroupRole>) -> ModelSnapshot;
async fn delete_group(id: String) -> ModelSnapshot;   // fails if the group holds a role
async fn move_item(id: String, to_group: String, new_order: i32) -> ModelSnapshot;
async fn rename_item(id: String, label: String) -> ModelSnapshot;
async fn set_item_icon(id: String, png: Vec<u8>) -> ModelSnapshot;
async fn remove_item(id: String) -> ModelSnapshot;
async fn sort_group(id: String) -> ModelSnapshot;
// queries
async fn search_all_apps(query: String) -> Vec<SearchHit>;
async fn icon_for(app: AppRef, size_px: u32) -> Option<Vec<u8>>;
async fn wallpaper_png() -> Option<Vec<u8>>;
async fn home_group_id() -> Option<String>;
// actions
async fn launch_app(app: AppRef) -> Result<(), String>;
async fn launch_shortcut(id: String) -> Result<(), String>;
async fn uninstall_app(app: AppRef) -> Result<(), String>;
async fn begin_add_shortcut() -> Vec<ShortcutProvider>;
async fn create_shortcut(provider: ShortcutProvider, target_group_id: String) -> Result<(), String>;
// settings
async fn settings() -> Settings;
async fn update_settings(patch: SettingsPatch) -> Settings;
// events
fn events() -> Stream<CoreEvent>;
```

**`create_shortcut` takes the target group explicitly**, and this is a
correctness requirement, not a convenience. Creating a shortcut round-trips
through another app's activity, and when it returns, Dart's notion of "the page
I was on" may be stale. Dart therefore resolves the target group id *before*
calling, and Rust never infers it. This is precisely the defect the old app had
(it read a raw adapter position after the round-trip and filed the shortcut one
group over).

**One event stream**, because nearly every screen has to react to the same
underlying changes:

```rust
enum CoreEvent {
    ModelChanged,                       // groups, items, ordering, settings
    AppsChanged,                        // reconciliation finished; new/removed apps
    ShortcutCreated { item_id: Uuid },  // the provider activity returned one
    ShortcutCancelled,                  // the user backed out of the provider activity
    WallpaperChanged,                   // re-fetch and re-render the base layer
    HomePressed,                        // Dart decides: close drawer, or go to Home group
    BackPressed,
    Error(String),
}
```

Without `ShortcutCreated`/`ShortcutCancelled` the Add Shortcut flow has no way
to close its dialog or report a failure; without `WallpaperChanged` the base
layer goes stale.

`ModelSnapshot` is the whole view-relevant model in one object. Returning a full
snapshot from every mutation is a deliberate simplification at this scale: it
removes a class of stale-cache bugs and the model is a few kilobytes.

## 9. Testing

- **Rust unit tests** — model operations (add/move/reorder/rename/delete, flag
  exclusivity, default groups, previous-group restore after reinstall), the
  ordering rules, store round-trip and atomic-write behaviour, schema migration,
  corrupt-document recovery, `.desktop` parsing from fixture files (including
  quoting, field codes, `Terminal=true`, `NoDisplay`, locale names), and search
  ranking.
- **Rust platform tests** — the `fake` `AppSource` drives reconciliation tests
  for install/uninstall/reinstall and category assignment without a device.
- **Dart widget tests** — the pager controller's index mapping (the bug that
  must not return), grid rendering against a fake snapshot, settings applying to
  rendering, drawer behaviour.
- **Dart integration test on Linux** — boot the app against the `fake` source,
  exercise discovery → grid → reorder → launch, assert the launch call.
- **Manual on device** — Pixel 8 and the Android 14 tablet: first-run discovery,
  install/uninstall while running, home-button behaviour, icon quality, the
  wallpaper overlay, orientation, and the `.debug` build not stealing home.

## 10. What is left to the builder

Deliberately not pinned down, because a competent builder should decide:

- Exact FRB codegen layout and whether generated Dart is checked in (recommended:
  yes, so a fresh clone builds without the codegen toolchain).
- Icon cache strategy (in-memory LRU, optionally a disk cache under the OS cache
  directory) and icon pixel sizes per density.
- SVG rasterisation on Linux (a Rust crate, or fall back to a placeholder).
- Search ranking details (prefix > substring > subsequence) — the old app was
  case-insensitive substring, which is a fine floor.
- Widget tree structure, theming, animation curves, and the visual language —
  the old app's look is a starting point, not a specification.
- The exact shape of the About screen, and where "Restart Launcher" and
  "Factory Reset" live.
- Whether Linux custom-command shortcuts ship in v1 or v1.1.
- Whether `ModelSnapshot` should grow change-deltas later if it ever gets slow.
