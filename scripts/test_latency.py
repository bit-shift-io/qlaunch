import unittest

import latency


class LatencyTests(unittest.TestCase):
    def test_percentile_nearest_rank(self):
        self.assertEqual(latency.percentile([10, 20, 30], 0.95), 30)
        self.assertEqual(latency.percentile([10, 20, 30], 0.5), 20)
        self.assertEqual(latency.percentile([], 0.5), 0.0)

    def test_parse_ignores_zero_and_sentinel(self):
        text = "\n".join(
            [
                "16666666",
                "0 0 0",
                "1 1000000000 2",
                "3 9223372036854775807 4",
                "5 1010000000 6",
            ]
        )
        stats = latency.parse(text, 90)
        self.assertEqual(stats.frames, 1)
        self.assertAlmostEqual(stats.p50, 10.0)

    def test_parse_computes_intervals_and_budget(self):
        text = "\n".join(
            [
                "16666666",
                "0 1000000000 0",
                "0 1010000000 0",
                "0 1020000000 0",
                "0 1040000000 0",
            ]
        )
        stats = latency.parse(text, 90)
        self.assertEqual(stats.frames, 3)
        self.assertAlmostEqual(stats.p50, 10.0)
        self.assertAlmostEqual(stats.p95, 20.0)
        self.assertAlmostEqual(stats.maximum, 20.0)
        self.assertAlmostEqual(stats.over_share, 1 / 3)

    def test_histogram_uses_present2present_buckets(self):
        text = "\n".join(
            [
                "layerName = bitshift.qlaunch/.MainActivity#1",
                "totalFrames = 10",
                "latch2present histogram is as below:",
                "16ms=999 50ms=999",
                "present2present histogram is as below:",
                "0ms=0 16ms=8 33ms=1 50ms=1",
                "acquire2present histogram is as below:",
                "16ms=999",
            ]
        )
        stats = latency.parse(text, 60)
        self.assertEqual(stats.frames, 10)
        self.assertAlmostEqual(stats.p50, 16.0)
        self.assertAlmostEqual(stats.p95, 50.0)
        self.assertAlmostEqual(stats.p99, 50.0)
        self.assertAlmostEqual(stats.maximum, 50.0)
        self.assertAlmostEqual(stats.over_share, 0.2)

    def test_empty_histogram_is_zero_frames(self):
        text = "present2present histogram is as below:\n0ms=0"
        stats = latency.parse(text, 60)
        self.assertEqual(stats.frames, 0)
        self.assertAlmostEqual(stats.over_share, 0.0)

    def test_weighted_percentile(self):
        self.assertEqual(latency.weighted_percentile([16.0, 33.0], [9, 1], 0.5), 16.0)
        self.assertEqual(latency.weighted_percentile([16.0, 33.0], [9, 1], 0.95), 33.0)
        self.assertEqual(latency.weighted_percentile([], [], 0.5), 0.0)


if __name__ == "__main__":
    unittest.main()
