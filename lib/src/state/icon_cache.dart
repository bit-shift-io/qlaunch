import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:qlaunch/src/rust/model/icon.dart';
import 'package:qlaunch/src/state/launcher_backend.dart';
import 'package:qlaunch/src/ui/raw_rgba_image.dart';

/// Session-scoped source cache. Decoded images have a separate Flutter budget.
class IconCache {
  IconCache(
    this._backend, {
    this.maximumBytes = 32 << 20,
    this.maximumEntries = 1000,
  });

  final LauncherBackend _backend;
  final int maximumBytes;
  final int maximumEntries;
  final Map<_Key, _Entry> _entries = <_Key, _Entry>{};
  final _pending = <_Key, Future<ImageProvider?>>{};
  int _retainedBytes = 0;

  int get retainedBytes => _retainedBytes;
  int get entryCount => _entries.length;

  _Entry? _touch(_Key key) {
    final entry = _entries.remove(key);
    if (entry != null) _entries[key] = entry;
    return entry;
  }

  ImageProvider? peek(String package, String activity, int sizePx) =>
      _touch(('app', package, activity, sizePx))?.provider;

  ImageProvider? peekItem(String id, String reference, int sizePx) =>
      _touch(('item', id, reference, sizePx))?.provider;

  Future<ImageProvider?> resolve(
    String package,
    String activity,
    int sizePx,
  ) => _resolve(('app', package, activity, sizePx), () async {
    final image = await _backend.iconFor(package, activity, sizePx);
    if (image == null) return const _Entry(null, 0);
    final ImageProvider provider = switch (image.format) {
      IconFormat.png => ResizeImage(MemoryImage(image.bytes), width: sizePx),
      IconFormat.rgba => RawRgbaImage(image.bytes, image.width, image.height),
    };
    return _Entry(provider, image.bytes.lengthInBytes);
  });

  Future<ImageProvider?> resolveItem(String id, String reference, int sizePx) =>
      _resolve(('item', id, reference, sizePx), () async {
        final bytes = await _backend.itemIcon(id);
        if (bytes == null) return const _Entry(null, 0);
        final data = bytes is Uint8List ? bytes : Uint8List.fromList(bytes);
        return _Entry(
          ResizeImage(MemoryImage(data), width: sizePx),
          data.length,
        );
      });

  Future<ImageProvider?> _resolve(_Key key, Future<_Entry> Function() load) {
    final entry = _touch(key);
    if (entry != null) return SynchronousFuture(entry.provider);
    return _pending.putIfAbsent(key, () {
      // Identity, rather than a global generation, permits selective invalidation.
      late final Future<ImageProvider?> future;
      future = Future<_Entry>.sync(load)
          .then((entry) {
            if (identical(_pending[key], future)) {
              if (entry.bytes <= maximumBytes && maximumEntries > 0) {
                _entries[key] = entry;
                _retainedBytes += entry.bytes;
                while (_entries.length > maximumEntries ||
                    _retainedBytes > maximumBytes) {
                  _retainedBytes -= _entries.remove(_entries.keys.first)!.bytes;
                }
              }
            }
            return entry.provider;
          })
          .whenComplete(() {
            if (identical(_pending[key], future)) _pending.remove(key);
          });
      return future;
    });
  }

  void invalidateApp(String package, String activity) => _invalidate(
    (key) => key.$1 == 'app' && key.$2 == package && key.$3 == activity,
  );

  void invalidateItem(String id) =>
      _invalidate((key) => key.$1 == 'item' && key.$2 == id);

  /// Only the current physical size is useful after an icon-size/DPR change.
  void retainSize(int sizePx) => _invalidate((key) => key.$4 != sizePx);

  void _invalidate(bool Function(_Key) matches) {
    for (final key in _entries.keys.where(matches).toList()) {
      _retainedBytes -= _entries.remove(key)!.bytes;
    }
    _pending.removeWhere((key, _) => matches(key));
  }

  void clear() {
    _entries.clear();
    _pending.clear();
    _retainedBytes = 0;
  }
}

typedef _Key = (String, String, String, int);

class _Entry {
  const _Entry(this.provider, this.bytes);
  final ImageProvider? provider;
  final int bytes;
}
