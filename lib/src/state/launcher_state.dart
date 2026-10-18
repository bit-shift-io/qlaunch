import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:qlaunch/src/rust/api/api.dart' show ModelSnapshot;
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/settings.dart' show SettingsPatch;
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

  // Generations only move forward. A fetch's result is dropped only when a
  // newer one has already been adopted, so a late older result can never
  // overwrite newer data, while a failed newer fetch can't block an older one.
  int _started = 0;
  int _adopted = 0;

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
  ///
  /// Never throws: a failure arrives as [error], like any other fetch.
  Future<void> load({String? configRootOverride}) async {
    _sub ??= _backend.events().listen(_onEvent, onError: _fail);
    await _fetch(() => _backend.init(configRootOverride: configRootOverride));
  }

  void _onEvent(CoreEvent event) {
    switch (event) {
      case CoreEvent_ModelChanged():
      case CoreEvent_AppsChanged():
        unawaited(_fetch(_backend.snapshot));
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

  Future<void> _fetch(Future<ModelSnapshot> Function() fetch) async {
    final gen = ++_started;
    try {
      final snapshot = await fetch();
      if (gen < _adopted) return;
      _adopted = gen;
      _adopt(snapshot);
    } catch (e, st) {
      if (gen < _adopted) return;
      _fail(e, st);
    }
  }

  /// The platform icon for an app, or `null` if there is none or it cannot be
  /// read. A missing icon is not an app-level error: it renders a placeholder and
  /// does not set [error].
  ///
  /// A throw from the backend is a bug rather than a missing icon, so it is
  /// reported like any other failure, but it still yields `null` and leaves
  /// [error] alone.
  Future<Uint8List?> iconFor(
    String package,
    String activity,
    int sizePx,
  ) async {
    try {
      return await _backend.iconFor(package, activity, sizePx);
    } catch (e, st) {
      _report(e, st);
      return null;
    }
  }

  /// Launches an app. A failure is reported through [error], not thrown.
  Future<void> launchApp(String package, String activity) async {
    try {
      await _backend.launchApp(package, activity);
    } catch (e, st) {
      _fail(e, st);
    }
  }

  /// Applies a settings patch and adopts the snapshot the core returns.
  ///
  /// The core clamps and validates, so the value the UI shows afterwards is the
  /// one in that snapshot rather than the one asked for. A failure arrives as
  /// [error], never as a throw.
  Future<void> updateSettings(SettingsPatch patch) =>
      _fetch(() => _backend.updateSettings(patch));

  /// Surfaces a failure as state, and reports it so that a programming error
  /// is not hidden behind a one-line message.
  void _fail(Object error, StackTrace stack) {
    _error = error.toString();
    _report(error, stack);
    _notify();
  }

  void _report(Object error, StackTrace stack) {
    FlutterError.reportError(
      FlutterErrorDetails(
        exception: error,
        stack: stack,
        library: 'qlaunch launcher state',
      ),
    );
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
