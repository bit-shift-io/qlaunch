# QLaunch — Search screen redesign (with Frequent / Recent)

**Design document · 2026-10-19**

Status: awaiting approval. A reviewer reads this before you do; any questions it
raises come back to you in one round.

2026-10-21 update: search is implemented. The approved follow-up in
`../plans/2026-10-21-search-screen.md` supersedes this document's overlay,
history-strip and Home-return behavior: search hides the main foreground while
sharing its wallpaper, Frequent/Recent use the main pager and title strip with
at most three rows each, and Home from another app first restores the last tab.

This extends `2026-10-18-qlaunch-flutter-rust-design.md` (still the authority for
everything it covers). It changes §8.2's search panel and adds a small piece of
core state.

---

## 1. Intent and why

Today the Home-button search is a full-screen list that opens showing **every
app** and narrows as you type. Two problems:

1. Opening it dumps the whole app list before the user has asked for anything,
   which is noise — the old behaviour only made sense as a crude app drawer.
2. It does not look or feel like the rest of the launcher: it lists plain text
   rows instead of the icon + label cells the home screen uses.

The user wants the panel to earn its space: **nothing until you type**, a search
box that slides down and keeps focus, a settings gear within reach, and results
drawn as **the same cells as the home screen**. Before typing it should offer
what a launcher usually wants to show — **Frequent** and **Recent** apps, as a
two-tab strip like Nova's app drawer.

"Frequent" and "Recent" need data the core does not keep today: it records
nothing about what was launched. So part of this is teaching the core to
remember usage.

The user chose a **full-screen replacement** over a floating overlay: an overlay
over the grid was judged too cluttered.

## 2. Goals

- The search panel replaces the home screen (`HomeScreen` route) rather than
  floating over it, and slides its search box down on open.
- Before typing: a **Frequent | Recent** tab strip; no results, no app dump.
- A **gear** to the left of the box opens the launcher Settings.
- Focus stays in the text box from open; typing filters by the existing
  prefix/substring/subsequence match.
- Results and the Frequent/Recent sets render as **home-screen cells** (icon +
  label, honouring the `layout`, `icon_size`, grid columns, and the Font size
  text settings).
- The core records launches (count + last used), persists them, and answers
  "frequent" and "recent" queries.

## 3. Non-goals

- No change to launch behaviour, the match algorithm, or `search_all_apps`'s
  matching (only its empty-query behaviour changes; see §5).
- No "All apps" tab, no per-tab customisation, no learning/decay beyond simple
  counts and timestamps.
- No search history or query suggestions.
- No change to the grid, tab strip, or pips on the home screen (beyond reusing
  the cell widget).

## 4. Anti-goals

- Opening the panel must not show every app before a keystroke.
- "Frequent"/"Recent" must not be invented client-side or approximate: if the
  core cannot answer, the tabs are empty, not wrong.
- A downgrade must not lose the user's layout (see the persistence decision in
  §6).

## 5. The search screen

**Shape.** `AppSearch` is rebuilt as a full screen that sits where it does now
(over the home column, in `HomeScreen`'s `Stack`), so it replaces the launcher
visually. An opaque backdrop, as today.

**Layout, top to bottom**
1. A row that **slides down** on open (`SlideTransition`, ~200 ms): a **gear**
   `IconButton` on the left (opens `SettingsScreen`, same route the long-press
   uses), then the **text field** (autofocus, as today), then the close button.
2. Under the box:
   - **Query empty** — a **Frequent | Recent** tab strip (two simple tabs, no
     page pips), and the selected set drawn as a grid of cells.
   - **Query non-empty** — the results grid (the tabs give way to it).
3. An empty result set shows nothing (no "no matches" text is required, but one
   is acceptable if the builder prefers it).

**Cells.** Reuse `AppCell`. `AppGrid` already does the work for a group; this
screen needs the same grid over a *computed list* rather than a group, so its
grid-delegate/icon-cache part should be factored into a reusable widget (e.g.
`AppCells`) that both `AppGrid` and the search screen use. Cells take a
synthesised `Item` per result (id/label/app from the hit); tapping launches and
closes the panel (§8.2's existing behaviour), a long-press does nothing here.

**Focus.** The field autofocuses; closing the keyboard must not drop focus, and
opening the panel must not require a tap. (Carried over from today.)

**Back button.** While the panel is open, system Back closes it (the keyboard,
if up, dismisses first, as Android does anyway). This is the one exception to
§8.2's "Back at home does nothing", and without it a full-screen panel with only
a close button can trap the user.

**Gear.** Tapping the gear pushes `SettingsScreen` over the panel; the panel
stays mounted underneath, so returning restores the query, the selected tab and
focus. It does not close the search.

**Empty query.** "Empty" means empty after trimming, matching the core's
existing match (which trims): a field holding only spaces shows the tabs, not
zero results.

## 6. Core: usage tracking

**What is recorded.** Per application — keyed like everything else by
`(package, activity)` — a **launch count** (`u32`) and a **last-used time**
(unix milliseconds, `i64`).

**Where it lives.** A new `usage` field on the stored `Document`, next to
`settings`, with `#[serde(default)]` so a document written before this change
still loads. Held in `App` beside `model`/`settings`.

**Schema decision.** Keep `SCHEMA_VERSION` at 1. A defaulted field is a
backward-compatible extension: an older build must be able to read a document
this build wrote, or a downgrade would hit `UnsupportedSchema` and **quarantine
the whole layout**. Bumping to 2 is the "proper" path on paper and is rejected
for exactly that reason. (Recorded here so it is not "fixed" later by mistake.)

**Recording.** `launch_app` updates the entry and saves the document, so a
launch from the grid or from search both count. The update takes an explicit
`now_ms`, so the usage module is testable without a clock; the API supplies
`SystemTime::now()`. **Only a launch that succeeds counts** — an app that failed
to open must not pollute Frequent. A save that fails (a blocked store) is
surfaced through the existing `save_problem` path like any other save, never as
a failed launch.

**Queries.** Two new bridge functions, each `(limit: u32)`:
- `frequent_apps` — non-uninstalled items with a usage entry, ordered by count
  desc (tie-break: most recent, then label).
- `recent_apps` — ordered by last-used desc.

Both resolve usage entries back to the current `Item`s (for the label to show),
skip apps no longer present, and return app + label. They return a **new
`UsageHit { app, label }`** rather than `SearchHit`: a usage result is not a
match, and inventing a `MatchKind` for it would be misleading. Hidden apps are
included, matching what `search_all_apps` already does.

**Empty query.** `search_all_apps("")` returns an **empty** list, not every app.
(This is the one behaviour change to the existing function; its matching is
untouched.)

## 7. Constraints

- One `TextStyle` source for the fonts (`label_style.dart`); the new cells use
  `labelTextStyle` via `AppCell`, so they already follow the Font size settings.
- The bridge is generated: after adding the API functions, run the codegen and
  rebuild the host lib, or `test/core_smoke_test.dart` fails on a hash mismatch.
- Rust core stays platform-independent; time comes from `std::time`, not a
  platform call.
- Tests: core unit tests for the usage module and the queries; widget tests for
  the empty-query tabs, the gear, the slide-down, and cell rendering; the
  existing search tests updated for the new empty-query behaviour.

## 8. Approved choices

- Default tab on open: **Frequent**.
- The Frequent/Recent grid **honours the home screen layout** — same `layout`,
  `icon_size` and columns as the grid.
- The exact `limit` (a page's worth, ~24), the tab styling, and the slide
  curve/duration.
- Whether to factor `AppGrid`'s grid into a shared widget or duplicate a small
  delegate in the search screen.
- Pruning usage entries for apps that no longer exist (nice to have; not
  required).
- The tie-break details beyond "count desc, then recent, then label".
