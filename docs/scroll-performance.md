# Continuous scroller performance — 2026-10-22

## Findings and changes

- The initial on-device prototype was installed in debug mode. Use profile for
  measurements and release for normal use.
- Crossing a section changed HomeScreen's active group, rebuilding GroupScroller,
  recreating every section/row descriptor and the visible list's widget subtree.
  This caused repeatable rendering stalls even after icons were warm.
- GroupScroller now retains its cycle and list subtree until the model snapshot,
  group order or grid geometry changes. Navigation commands still run immediately.
- Each embedded app row previously created its own GridView/Scrollable/viewport.
  Embedded rows now use ordinary equal-width cells, including empty slots in the
  last row. The device's initial view went from six Scrollables to one, with the
  same 25 mounted cells. Icon lookup, launch and long-press handling are shared
  with the regular grids.
- Section headings now have a tight central slot and centered text. Equal-width
  dividers keep the title at the screen center, including short titles.

## Controlled device comparison

iPlay60 mini Turbo, Android 14, profile mode, real installed groups and icons
(58 items, 3 groups). Twelve flings per pass, then twelve section-tab taps.
The following A/B isolates **list reuse**: both runs already use lightweight app
rows; the baseline temporarily disabled reuse. All numbers are milliseconds.

| Metric | Reuse disabled | Reuse enabled |
| --- | ---: | ---: |
| Warm scroll average UI | 1.047 | 0.768 |
| Warm scroll p99 UI | 11.112 | 3.231 |
| Warm scroll average raster | 6.059 | 4.571 |
| Warm scroll p99 raster | 49.514 | 15.746 |
| Warm scroll raster budget misses | 52 / 1213 | 13 / 1288 |
| Section navigation average raster | 7.417 | 4.463 |
| Section navigation p99 raster | 49.296 | 14.829 |
| Section navigation raster budget misses | 28 / 302 | 1 / 354 |

At 60 Hz, one frame has about 16.7 ms. These measurements show substantially
smaller stalls, not a guarantee that every frame meets its deadline.

Disabling the fade reduced the optimized average scroll raster time from 4.571
to 3.871 ms and p99 from 15.746 to 15.382 ms. It is a secondary cost; the fade
remains available. Warm and initial passes are similar, so cold icon loading
does not explain the repeated stalls in this workload.

## Reproduce

```sh
JAVA_HOME=/opt/jbr-21 PATH="/opt/jbr-21/bin:$PATH" \
  flutter drive --profile --no-dds -d DEVICE \
  --driver=test_driver/performance.dart \
  --target=integration_test/scroll_performance_test.dart
```

The benchmark uses `fullyLive` frame policy: animations render on normal engine
frames rather than only test pumps. Earlier exploratory runs with the default
test policy are not the results in the table above. `--no-dds` permits the
on-device integration binding to collect timeline/GC information.

It temporarily enables continuous scrolling, measures initial and warm passes,
section navigation, and a no-fade pass; teardown restores the original mode and
fade settings. Full metrics are written to `build/integration_response_data.json`.
The default warm-scroll raster p99 limit is 25 ms (1.5 frames at 60 Hz); override
with `--dart-define=MAX_SCROLL_RASTER_P99_MS=...` for other performance targets.

## Further work if larger app libraries expose more stalls

- Profile the remaining row-entry frames before adding more caches. Ordinary
  cells still measure label geometry individually; sharing those measurements
  is a candidate if UI work becomes dominant.
- The icon warm queue is rebuilt on section changes. Defer/reprioritize this
  work until scrolling stops if larger libraries make it significant.
- For very long sections or distant jumps, measure a varied-extent sliver with
  known heading/row extents instead of SliverList's dead-reckoning layout.
- A cheaper edge-only fade is worth considering if GPU fill/compositing becomes
  the limiting factor; the current A/B does not justify removing the effect.
