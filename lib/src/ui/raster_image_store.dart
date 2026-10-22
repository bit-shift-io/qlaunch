import 'dart:ui' as ui;

import 'package:qlaunch/src/ui/raster_cache_stats.dart';

/// Identity of pixels, independent of the widget currently displaying them.
class RasterCacheKey {
  const RasterCacheKey({
    required this.generation,
    required this.kind,
    required this.owner,
    required this.pixels,
  });
  final Object generation;
  final String kind, owner;
  final Object pixels;

  @override
  bool operator ==(Object other) =>
      other is RasterCacheKey &&
      generation == other.generation &&
      kind == other.kind &&
      owner == other.owner &&
      pixels == other.pixels;
  @override
  int get hashCode => Object.hash(generation, kind, owner, pixels);
}

class _ImageEntry {
  _ImageEntry(
    this.key,
    this.image,
    this.pixelRatio,
    this.logicalSize,
    this.sequence,
  ) : bytes = image.width * image.height * 4;
  final RasterCacheKey key;
  final ui.Image image;
  final double pixelRatio;
  final ui.Size logicalSize;
  final int sequence;
  final int bytes;
  int references = 0;
  bool retired = false;
  bool discardOnRelease = false;
}

/// A displayed image cannot be freed until every consumer releases its lease.
class RasterImageLease {
  RasterImageLease._(this._store, this._entry);
  final RasterImageStore _store;
  final _ImageEntry _entry;
  bool _released = false;
  ui.Image get image => _entry.image;
  double get pixelRatio => _entry.pixelRatio;
  ui.Size get logicalSize => _entry.logicalSize;
  RasterCacheKey get key => _entry.key;
  bool get retired => _entry.retired;
  void release() {
    if (_released) return;
    _released = true;
    _store._release(_entry);
  }
}

/// Session-owned full-resolution images. The byte target is soft while pinned.
/// No listeners/notifications: accounting must not rebuild the widget tree.
class RasterImageStore {
  RasterImageStore({
    this.maximumBytes = 64 << 20,
    this.maximumEntries = 4096,
    this.recentGenerations = 2,
  }) {
    if (maximumBytes < 0 || maximumEntries < 0 || recentGenerations < 0) {
      throw ArgumentError('Raster cache limits must be nonnegative');
    }
  }
  final int maximumBytes, maximumEntries, recentGenerations;
  final _entries = <RasterCacheKey, _ImageEntry>{};
  final _generations = <Object>{};
  Object? _current;
  int _retained = 0, _pinned = 0, _retired = 0, _peak = 0;
  int _hits = 0, _misses = 0, _captures = 0, _evictions = 0;
  int _sequence = 0;
  int _failures = 0, _captureMicros = 0, _captureMaxMicros = 0, _pruned = 0;

  Object? get currentGeneration => _current;

  /// Superseded revisions removed; deliberately not counted as evictions.
  int get prunedRevisions => _pruned;
  RasterCacheStats get statistics => RasterCacheStats(
    entryCount: _entries.length,
    retainedBytes: _retained,
    pinnedBytes: _pinned,
    retiredLiveBytes: _retired,
    peakLiveBytes: _peak,
    hits: _hits,
    misses: _misses,
    captures: _captures,
    evictions: _evictions,
    captureFailures: _failures,
    captureMicros: _captureMicros,
    captureMaxMicros: _captureMaxMicros,
  );
  void resetStatistics() {
    _hits = _misses = _captures = _evictions = _failures = _pruned = 0;
    _captureMicros = _captureMaxMicros = 0;
    _peak = _retained + _retired;
  }

  void recordCapture({required int microseconds, bool failed = false}) {
    if (failed) _failures++;
    _captureMicros += microseconds;
    if (microseconds > _captureMaxMicros) _captureMaxMicros = microseconds;
  }

  void activateGeneration(Object generation) {
    if (_current == generation) return;
    _current = generation;
    _generations.remove(generation);
    _generations.add(generation);
    while (_generations.length > recentGenerations + 1) {
      _generations.remove(_generations.first);
    }
    _trim(pruneGenerations: true);
  }

  bool contains(RasterCacheKey key) => _entries.containsKey(key);
  RasterImageLease? acquire(RasterCacheKey key) {
    final entry = _entries.remove(key);
    if (entry == null) {
      _misses++;
      return null;
    }
    _hits++;
    _entries[key] = entry;
    entry.discardOnRelease = false;
    return _pin(entry);
  }

  RasterImageLease _pin(_ImageEntry entry) {
    if (entry.references++ == 0 && !entry.retired) _pinned += entry.bytes;
    return RasterImageLease._(this, entry);
  }

  /// Transfers ownership and pins before trimming, including oversized images.
  RasterImageLease insertAndAcquire(
    RasterCacheKey key,
    ui.Image image,
    double pixelRatio,
    ui.Size logicalSize,
  ) {
    if (!pixelRatio.isFinite ||
        pixelRatio <= 0 ||
        logicalSize.isEmpty ||
        !logicalSize.width.isFinite ||
        !logicalSize.height.isFinite) {
      image.dispose();
      throw ArgumentError('Invalid raster image geometry');
    }
    if (_current == null) activateGeneration(key.generation);
    final previous = _entries.remove(key);
    if (previous != null) _retire(previous);
    final entry = _ImageEntry(key, image, pixelRatio, logicalSize, _sequence++);
    _entries[key] = entry;
    // A newer revision of one owner's pixels supersedes its unpinned older
    // revisions. A different live geometry could still be on screen, so pinned
    // variants wait for their lease.
    for (final other
        in _entries.keys
            .where(
              (other) =>
                  other != key &&
                  other.generation == key.generation &&
                  other.kind == key.kind &&
                  other.owner == key.owner &&
                  _entries[other]!.references == 0 &&
                  _entries[other]!.sequence < entry.sequence,
            )
            .toList()) {
      _prune(other);
    }
    _retained += entry.bytes;
    _captures++;
    final lease = _pin(entry);
    final live = _retained + _retired;
    if (live > _peak) _peak = live;
    _trim();
    return lease;
  }

  void _release(_ImageEntry entry) {
    if (--entry.references != 0) return;
    if (entry.retired) {
      _retired -= entry.bytes;
      entry.image.dispose();
    } else {
      _pinned -= entry.bytes;
      if (entry.discardOnRelease ||
          !_generations.contains(entry.key.generation) ||
          _superseded(entry.key)) {
        _entries.remove(entry.key);
        _retire(entry);
      }
      _trim();
    }
  }

  bool _superseded(RasterCacheKey key) {
    final entry = _entries[key];
    if (entry == null) return false;
    return _entries.values.any(
      (other) =>
          other != entry &&
          other.sequence > entry.sequence &&
          other.key.generation == key.generation &&
          other.key.kind == key.kind &&
          other.key.owner == key.owner,
    );
  }

  void _retire(_ImageEntry entry) {
    _retained -= entry.bytes;
    entry.retired = true;
    if (entry.references > 0) {
      _pinned -= entry.bytes;
      _retired += entry.bytes;
    } else {
      entry.image.dispose();
    }
  }

  /// Removes a superseded revision. Not a capacity eviction, so the warm-up
  /// "0 evictions" gate keeps meaning.
  void _prune(RasterCacheKey key) {
    final entry = _entries.remove(key);
    if (entry == null) return;
    _retire(entry);
    _pruned++;
  }

  void _evict(RasterCacheKey key) {
    final entry = _entries.remove(key)!;
    _retire(entry);
    _evictions++;
  }

  void _trim({bool pruneGenerations = false}) {
    // Old generations are pruned even when there is no byte pressure. Pinned
    // entries stay discoverable and are reconsidered on their last release.
    if (pruneGenerations) {
      for (final key
          in _entries.keys
              .where(
                (key) =>
                    !_generations.contains(key.generation) &&
                    _entries[key]!.references == 0,
              )
              .toList()) {
        _evict(key);
      }
    }
    while (_retained + _retired > maximumBytes ||
        _entries.length > maximumEntries) {
      RasterCacheKey? candidate;
      for (final entry in _entries.values) {
        if (entry.references == 0 && entry.key.generation != _current) {
          candidate = entry.key;
          break;
        }
      }
      if (candidate == null) {
        for (final entry in _entries.values) {
          if (entry.references == 0) {
            candidate = entry.key;
            break;
          }
        }
      }
      if (candidate == null) break;
      _evict(candidate);
    }
  }

  void invalidateOwner(String owner) {
    for (final key in _entries.keys.where((k) => k.owner == owner).toList()) {
      _retire(_entries.remove(key)!);
    }
  }

  void retainOwners(Set<String> owners) {
    for (final key
        in _entries.keys.where((k) => !owners.contains(k.owner)).toList()) {
      _retire(_entries.remove(key)!);
    }
  }

  bool canWarm({required RasterCacheKey key, required int estimatedBytes}) {
    if (key.generation != _current || contains(key) || estimatedBytes <= 0)
      return false;
    var reclaimableBytes = 0, reclaimableEntries = 0;
    for (final entry in _entries.values) {
      if (entry.references == 0 && entry.key.generation != _current) {
        reclaimableBytes += entry.bytes;
        reclaimableEntries++;
      }
    }
    return _retained + _retired - reclaimableBytes + estimatedBytes <=
            maximumBytes &&
        _entries.length - reclaimableEntries + 1 <= maximumEntries;
  }

  void trimForMemoryPressure() {
    for (final entry in _entries.values.toList()) {
      if (entry.references == 0) {
        _evict(entry.key);
      } else {
        entry.discardOnRelease = true;
      }
    }
  }

  void clear() {
    for (final entry in _entries.values) {
      _retire(entry);
    }
    _entries.clear();
    _generations.clear();
    _current = null;
  }
}
