/// Pixel-payload estimates, not graphics-driver allocations or process memory.
/// Pinned bytes are a subset of retained bytes, not an additional allocation.
class RasterCacheStats {
  const RasterCacheStats({
    required this.entryCount,
    required this.retainedBytes,
    required this.pinnedBytes,
    required this.retiredLiveBytes,
    required this.peakLiveBytes,
    required this.hits,
    required this.misses,
    required this.captures,
    required this.evictions,
    required this.captureFailures,
    required this.captureMicros,
    required this.captureMaxMicros,
  });

  final int entryCount;
  final int retainedBytes;
  final int pinnedBytes;
  final int retiredLiveBytes;
  final int peakLiveBytes;
  final int hits;
  final int misses;
  final int captures;
  final int evictions;
  final int captureFailures;
  final int captureMicros;
  final int captureMaxMicros;
  int get liveBytes => retainedBytes + retiredLiveBytes;

  Map<String, Object> toJson() => {
    'entries': entryCount,
    'retainedBytes': retainedBytes,
    'pinnedBytes': pinnedBytes,
    'retiredLiveBytes': retiredLiveBytes,
    'liveBytes': liveBytes,
    'peakLiveBytes': peakLiveBytes,
    'hits': hits,
    'misses': misses,
    'captures': captures,
    'evictions': evictions,
    'captureFailures': captureFailures,
    'captureMicros': captureMicros,
    'captureMaxMicros': captureMaxMicros,
  };
}
