#!/usr/bin/env python3
"""Summarize SurfaceFlinger presentation intervals.

Two dump formats are accepted:

* ``dumpsys SurfaceFlinger --latency``: each row is ``desiredPresentTime
  actualPresentTime frameReadyTime`` in nanoseconds. We keep the actual
  presentation time, sort the frames, and diff consecutive presents. The first
  line of a dump is the refresh period, not a frame.
* ``dumpsys SurfaceFlinger --timestats -dump`` for one layer: the
  ``present2present histogram`` counts presents per millisecond bucket. Some
  Android 14 builds disable the ``--latency`` sampler (it returns only the
  refresh period) but still collect TimeStats. Histogram percentiles are
  bucket values, so they are accurate to the bucket width (1 ms).
"""

import argparse
import math
import os
import re
import statistics
from dataclasses import dataclass

SENTINEL = 2**63 - 1


def percentile(sorted_values, q):
    """Nearest-rank percentile of an already sorted sequence."""
    if not sorted_values:
        return 0.0
    q = min(max(q, 0.0), 1.0)
    rank = math.ceil(q * len(sorted_values))
    return sorted_values[max(rank - 1, 0)]


@dataclass
class Stats:
    frames: int
    p50: float
    p95: float
    p99: float
    maximum: float
    over_share: float


def parse(text, refresh_hz=90.0):
    """Parse one SurfaceFlinger dump."""
    histogram = parse_histogram(text, refresh_hz)
    if histogram is not None:
        return histogram

    stamps = []
    for line in text.splitlines()[1:]:
        parts = line.split()
        if len(parts) < 2:
            continue
        try:
            actual = int(parts[1])
        except ValueError:
            continue
        if actual in (0, SENTINEL):
            continue
        stamps.append(actual)

    stamps.sort()
    intervals = []
    for previous, current in zip(stamps, stamps[1:]):
        ms = (current - previous) / 1e6
        if 0.0 < ms <= 1000.0:
            intervals.append(ms)
    intervals.sort()
    return summarize(intervals, refresh_hz)


HISTOGRAM_MARKER = "present2present histogram is as below:"
BUCKET = re.compile(r"(\d+)ms=(\d+)")


def histogram_buckets(text):
    """Return ``[(bucket_ms, count)]`` for the first present2present histogram."""
    lines = text.splitlines()
    try:
        start = next(
            index
            for index, line in enumerate(lines)
            if HISTOGRAM_MARKER in line
        )
    except StopIteration:
        return None
    for line in lines[start + 1 :]:
        matches = BUCKET.findall(line)
        if matches:
            return [(float(value), int(count)) for value, count in matches]
        if line.strip():
            break
    return None


def parse_histogram(text, refresh_hz=90.0):
    """Summarize a TimeStats histogram dump, or return ``None`` if absent."""
    buckets = histogram_buckets(text)
    if buckets is None:
        return None
    values = [bucket for bucket, count in buckets if count > 0 and bucket > 0.0]
    counts = [count for bucket, count in buckets if count > 0 and bucket > 0.0]
    if not values:
        return summarize([], refresh_hz)

    budget = 1000.0 / refresh_hz
    over = sum(count for value, count in zip(values, counts) if value > 1.5 * budget)
    total = sum(counts)
    return Stats(
        frames=total,
        p50=weighted_percentile(values, counts, 0.50),
        p95=weighted_percentile(values, counts, 0.95),
        p99=weighted_percentile(values, counts, 0.99),
        maximum=max(value for value, count in zip(values, counts) if count > 0),
        over_share=over / total,
    )


def weighted_percentile(values, counts, q):
    """Nearest-rank percentile of a value histogram."""
    total = sum(counts)
    if total == 0:
        return 0.0
    target = math.ceil(min(max(q, 0.0), 1.0) * total)
    seen = 0
    for value, count in zip(values, counts):
        seen += count
        if seen >= target:
            return value
    return values[-1]


def summarize(intervals, refresh_hz):
    budget = 1000.0 / refresh_hz
    over = sum(1 for ms in intervals if ms > 1.5 * budget)
    return Stats(
        frames=len(intervals),
        p50=percentile(intervals, 0.50),
        p95=percentile(intervals, 0.95),
        p99=percentile(intervals, 0.99),
        maximum=intervals[-1] if intervals else 0.0,
        over_share=(over / len(intervals)) if intervals else 0.0,
    )


def format_stats(name, stats):
    return (
        f"{name}: frames={stats.frames} p50={stats.p50:.2f}ms "
        f"p95={stats.p95:.2f}ms p99={stats.p99:.2f}ms max={stats.maximum:.2f}ms "
        f"over={stats.over_share * 100.0:.1f}%"
    )


def median_stats(results):
    return Stats(
        frames=int(statistics.median([r.frames for r in results])),
        p50=statistics.median([r.p50 for r in results]),
        p95=statistics.median([r.p95 for r in results]),
        p99=statistics.median([r.p99 for r in results]),
        maximum=statistics.median([r.maximum for r in results]),
        over_share=statistics.median([r.over_share for r in results]),
    )


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--refresh", type=float, default=90.0, help="display refresh rate in Hz")
    parser.add_argument("files", nargs="+", help="SurfaceFlinger --latency dumps")
    args = parser.parse_args(argv)

    results = []
    for path in args.files:
        with open(path, encoding="utf-8") as handle:
            stats = parse(handle.read(), args.refresh)
        results.append(stats)
        print(format_stats(os.path.basename(path), stats))

    if len(results) > 1:
        print(format_stats("median", median_stats(results)))


if __name__ == "__main__":
    main()
