import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:qlaunch/src/rust/api/api.dart';
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/state/launcher_backend.dart';

/// A cache over the core's `snapshot()`. Everything it reports was computed by
/// Rust; it applies no ordering or role rule of its own.
class LauncherState extends ChangeNotifier {
  LauncherState(this._backend);

  final LauncherBackend _backend;
  StreamSubscription<CoreEvent>? _sub;
  ModelSnapshot? _snapshot;
  String? _error;
  bool _disposed = false;

  ModelSnapshot? get snapshot => _snapshot;

  /// The latest platform or re-fetch failure, cleared by the next successful
  /// fetch.
  String? get error => _error;

  List<String> get pageIds => _snapshot?.pageIds ?? const [];

  List<Group> get groups => _snapshot?.groups ?? const [];

  /// A filter only: the snapshot's items are already in display order.
  List<Item> itemsIn(String groupId) {
    final s = _snapshot;
    if (s == null) return const [];
    return s.items.where((i) => i.groupId == groupId).toList();
  }

  Group? get homeGroup {
    final s = _snapshot;
    final id = s?.homeGroupId;
    if (s == null || id == null) return null;
    for (final g in s.groups) {
      if (g.id == id) return g;
    }
    return null;
  }

  /// Subscribes to core events *before* fetching: the core registers its
  /// subscriber asynchronously, so fetching first could miss a change made
  /// in between. Only ever subscribes once, however often this is called.
  Future<void> load({String? configRootOverride}) async {
    _sub ??= _backend.events().listen(_onEvent);
    _adopt(await _backend.init(configRootOverride: configRootOverride));
  }

  void _onEvent(CoreEvent event) {
    switch (event) {
      case CoreEvent_ModelChanged():
      case CoreEvent_AppsChanged():
        unawaited(_refetch());
      case CoreEvent_Error(:final field0):
        _error = field0;
        _notify();
      // The spec defers shortcuts, wallpaper and the system button events, so
      // v1 does nothing with them. Listed rather than defaulted so a variant
      // added later is a compile error here.
      case CoreEvent_ShortcutCreated():
      case CoreEvent_ShortcutCancelled():
      case CoreEvent_WallpaperChanged():
      case CoreEvent_HomePressed():
      case CoreEvent_BackPressed():
        break;
    }
  }

  Future<void> _refetch() async {
    try {
      _adopt(await _backend.snapshot());
    } catch (e) {
      _error = e.toString();
      _notify();
    }
  }

  void _adopt(ModelSnapshot snapshot) {
    _snapshot = snapshot;
    _error = null;
    _notify();
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    // Not awaited: cancelling an `events()` stream does not complete until the
    // next event arrives, which may be never.
    unawaited(_sub?.cancel());
    super.dispose();
  }
}
