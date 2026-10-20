import 'package:flutter/painting.dart';
import 'package:qlaunch/src/rust/model/icon.dart';
import 'package:qlaunch/src/state/launcher_backend.dart';
import 'package:qlaunch/src/ui/raw_rgba_image.dart';

/// A cache of icon [ImageProvider]s, one per `(package, activity, size_px)`,
/// that lives for the whole session.
///
/// It sits above the widgets on purpose: the grid's pages are disposed and
/// rebuilt as the user pages through the launcher, and a per-widget cache loses
/// its icons every time it is thrown away. Flutter's own `ImageCache` caches the
/// decoded image behind each provider, so a provider resolved once is decoded
/// once per session.
class IconCache {
  IconCache(this._backend);

  final LauncherBackend _backend;
  final Map<_Key, Future<ImageProvider?>> _providers = {};

  /// The provider for an app's icon at [sizePx], or `null` if the app has none.
  ///
  /// Concurrent and repeated calls for one key share a single backend call. A
  /// failed call is not cached, so the next resolve tries the backend again.
  Future<ImageProvider?> resolve(String package, String activity, int sizePx) {
    final key = (package: package, activity: activity, sizePx: sizePx);
    return _providers.putIfAbsent(key, () async {
      try {
        return await _load(package, activity, sizePx);
      } catch (_) {
        _providers.remove(key);
        rethrow;
      }
    });
  }

  Future<ImageProvider?> _load(
    String package,
    String activity,
    int sizePx,
  ) async {
    final image = await _backend.iconFor(package, activity, sizePx);
    if (image == null) return null;
    return switch (image.format) {
      // A PNG is decoded at display size rather than at its intrinsic size.
      IconFormat.png => ResizeImage(MemoryImage(image.bytes), width: sizePx),
      // Raw pixels are already the size requested; decode them as-is.
      IconFormat.rgba => RawRgbaImage(image.bytes, image.width, image.height),
    };
  }

  /// Drops every cached provider, so the next resolve asks the backend again.
  /// Called when the installed apps change, since their icons may have too.
  void clear() => _providers.clear();
}

typedef _Key = ({String package, String activity, int sizePx});
