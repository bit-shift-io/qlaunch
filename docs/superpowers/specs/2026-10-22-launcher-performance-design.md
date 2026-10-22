# Launcher performance and battery optimisation

## Intent

Make page revisits paint already-decoded icons, including icons from every page
when they fit the memory budget. Reduce redundant icon fetching, image creation,
directory scans, package scans and snapshot work without changing appearance.

The user requested a plan and implementation after a code audit. This document
records implementation choices, not measured performance claims.

## First delivery: icons and background work

- Keep the existing Flutter UI, Rust core, Kotlin host, and raw RGBA Android path.
- Cache application and shortcut providers in one session-scoped cache. Use LRU
  eviction with a **32 MiB source-byte budget** and **1000-entry cap**, including
  missing results. Deduplicate in-flight requests separately from retained entries.
- Set Flutter's decoded-image cache to **64 MiB and 1000 entries**, once at startup.
  This is a cache budget, not a guarantee about total process/GPU memory: live
  widgets, transient decoding and engine allocations consume additional memory.
- Resolve providers to actual images during warming; fetching bytes alone is not
  warming. Prefer current-page items, then adjacent groups, then remaining page
  groups. Warm all icons if they fit, but do not repeatedly cycle through a working
  set larger than either cache. Stop speculative warming when the budget is reached.
- Use one background warm-up operation at a time. Schedule each next operation at
  scheduler idle priority, without polling or forcing animation frames. A visible
  widget's request is never blocked behind the speculative queue.
- Pause new speculative work during scrolling, while a pushed route/search covers
  the home screen, and whenever lifecycle is not resumed. Cancel obsolete queued
  work on settings/size/snapshot changes and disposal. Already-running backend work
  may complete, but must not repopulate invalidated entries or continue the queue.
- On memory pressure, cancel warming and release unused provider references and
  Flutter's non-live decoded cache. Do not force eviction of images still displayed.
  Resume warming only after subsequent user navigation or relevant model changes.
- Reuse cached results synchronously when building cells; avoid creating a fresh
  async wrapper and waiting frame on every rebuild. Existing visible images must
  not flash placeholders when unrelated launcher state changes.
- Shortcut icons use the same bounded cache, display-sized decoding and warming.
  Invalidate on item removal or icon reference changes; explicit icon-setting
  mutations invalidate the affected item even if its filename stays the same.
- Expose installed-app icon revisions through snapshots. Updating/removing one app
  invalidates only its Dart providers. Existing disk keys already include revision;
  keep other disk entries and allow bounded maintenance to remove obsolete entries.
- Replace per-icon full-directory maintenance with shared, serialised accounting:
  scan at cache construction/first use, maintain totals during writes/deletes, and
  evict oldest entries only when necessary. Handle concurrent writes and clears
  without stale resurrection or shared temporary filenames.
- Coalesce Android package broadcasts using one worker and a trailing **250 ms**
  quiet window. Retain and finish every `goAsync()` result, including failures;
  never hold a broadcast indefinitely under continuous events (flush by **1 s**).
- Coalesce overlapping event-driven snapshot requests. Preserve generation safety
  and external changes; do not suppress events merely because a mutation is active.
- Index items by group once per adopted snapshot. Unrelated Home/Back notifications
  must not rescan and enqueue every icon.

## Separate follow-up: usage persistence

Every successful launch currently serialises, validates, writes and syncs the full
configuration under the global Rust lock. This deserves its own storage change,
not an incidental cache edit. Preserve the existing durable behaviour in the first
delivery. A follow-up should evaluate a compact durable usage journal versus a
separate usage store, including migration, backup/restore/reset and crash recovery.
Do not silently introduce delayed writes or lost launch history.

## Non-goals

No full-page screenshots, appearance changes, new caching dependencies, compressed
Android transport, per-frame bitmap streaming, unbounded caches, periodic timers
while idle, or promises that decoding also preuploads all GPU textures. Do not
change the configuration schema in the first delivery. Do not add repaint layers
without measurements.

## Verification and success

Automated tests must demonstrate actual decoded-cache population, bounded retained
bytes, deduplication, selective invalidation, cancellation, lifecycle gating,
memory-pressure handling, shortcut reuse and concurrent disk correctness. Existing
Flutter and Rust tests must remain green.

Device verification needs a connected Android device: compare profile-mode frame
timings for cold/warm horizontal swipes, vertical scrolling, search and app updates;
record cache statistics and memory before/after warming and pressure. Check idle,
background and package-update CPU activity with Perfetto; assess battery using
repeatable runs, not a short battery-percentage observation. A Pixel 8 was connected
after the initial audit and is available for verification. Preserve its original
launcher configuration before tests and restore it afterwards; never uninstall or
clear app data. Its installed release rejects `run-as`, so use a verified built-in
configuration export and preserve private shortcut assets through in-place upgrades.
