import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:qlaunch/src/rust/api/api.dart' show ModelSnapshot;
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/api/shortcuts.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/search.dart' show SearchHit;
import 'package:qlaunch/src/rust/settings.dart' show SettingsPatch;
import 'package:qlaunch/src/rust/usage.dart' show UsageHit;
import 'package:qlaunch/src/state/icon_cache.dart';
import 'package:qlaunch/src/state/launcher_backend.dart';

/// A cache over the core's `snapshot()`. Everything it reports was computed by
/// Rust; it applies no ordering or role rule of its own.
class LauncherState extends ChangeNotifier {
  LauncherState(this._backend);

  final LauncherBackend _backend;

  /// Decoded-icon providers, shared across every screen and surviving the
  /// grid's page widgets being disposed and rebuilt.
  late final IconCache _icons = IconCache(_backend);

  StreamSubscription<CoreEvent>? _sub;
  ModelSnapshot? _snapshot;
  String? _error;
  int _homeRequests = 0;
  int _backRequests = 0;
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

  /// How many times the system Home button has been pressed since the launcher
  /// started. A counter rather than a flag: a listener compares it against the
  /// value it last saw, so it reacts to every press and never to a rebuild.
  int get homeRequests => _homeRequests;

  /// How many times the system Back button has been pressed since the launcher
  /// started. A counter, like [homeRequests], so a listener reacts to each
  /// press rather than to every rebuild.
  int get backRequests => _backRequests;

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

  /// The group holding the Apps role, or `null` if none does. Every app has a
  /// home here when it is unticked from another group.
  Group? get appsGroup {
    for (final g in groups) {
      if (g.role == GroupRole.apps) return g;
    }
    return null;
  }

  /// The page groups in page order. `pageIds` is the order the core computed.
  List<Group> get pages {
    final s = _snapshot;
    if (s == null) return const [];
    final byId = {for (final g in s.groups) g.id: g};
    return [
      for (final id in s.pageIds)
        if (byId[id] != null) byId[id]!,
    ];
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
        unawaited(_fetch(_backend.snapshot));
      case CoreEvent_AppsChanged():
        // An app may have been installed, removed or updated, so any icon we
        // cached for it could be stale.
        _icons.clear();
        unawaited(_fetch(_backend.snapshot));
      case CoreEvent_Error(:final field0):
        _error = field0;
        _notify();
      case CoreEvent_HomePressed():
        _homeRequests++;
        _notify();
      case CoreEvent_BackPressed():
        _backRequests++;
        _notify();
      // The window shows the system wallpaper itself, so a change needs no
      // action here; shortcuts are deferred. Listed rather than defaulted so a
      // variant added later is a compile error here.
      case CoreEvent_WallpaperChanged():
      case CoreEvent_ShortcutCreated():
      case CoreEvent_ShortcutCancelled():
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
  Future<ImageProvider?> iconFor(
    String package,
    String activity,
    int sizePx,
  ) async {
    try {
      return await _icons.resolve(package, activity, sizePx);
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

  /// Searches every app for [query], ranked by the core. An empty query
  /// returns nothing, not every app: the search screen shows its own
  /// Frequent/Recent lists before the user types.
  ///
  /// A failure yields no hits rather than an error: search is ephemeral, and a
  /// stale message under the field would be noise. A throw is still reported,
  /// since it means the bridge is broken.
  Future<List<SearchHit>> searchAllApps(String query) async {
    try {
      return await _backend.searchAllApps(query);
    } catch (e, st) {
      _report(e, st);
      return const [];
    }
  }

  /// The most-launched apps, up to [limit], as the core ranks them. A failure
  /// yields no hits and is reported, exactly like [searchAllApps].
  Future<List<UsageHit>> frequentApps(int limit) async {
    try {
      return await _backend.frequentApps(limit);
    } catch (e, st) {
      _report(e, st);
      return const [];
    }
  }

  /// The most-recently-launched apps, up to [limit]. A failure behaves as in
  /// [frequentApps].
  Future<List<UsageHit>> recentApps(int limit) async {
    try {
      return await _backend.recentApps(limit);
    } catch (e, st) {
      _report(e, st);
      return const [];
    }
  }

  /// Applies a settings patch and adopts the snapshot the core returns.
  ///
  /// The core clamps and validates, so the value the UI shows afterwards is the
  /// one in that snapshot rather than the one asked for. A failure arrives as
  /// [error], never as a throw.
  Future<void> updateSettings(SettingsPatch patch) =>
      _fetch(() => _backend.updateSettings(patch));

  Future<void> factoryReset() => _fetch(_backend.factoryReset);

  /// Writes the layout to a dated backup in Downloads and returns its file
  /// name, or `null` when the write failed (reported through [error]).
  Future<String?> backupConfig() async {
    try {
      return await _backend.backupConfig();
    } catch (e, st) {
      _fail(e, st);
      return null;
    }
  }

  /// The backups already in Downloads, newest first. A failure yields none; the
  /// error is reported.
  Future<List<String>> listBackups() async {
    try {
      return await _backend.listBackups();
    } catch (e, st) {
      _report(e, st);
      return const [];
    }
  }

  /// Replaces everything with the named backup. A failure arrives as [error],
  /// never as a throw, and changes nothing.
  Future<void> restoreConfig(String fileName) =>
      _fetch(() => _backend.restoreConfig(fileName));

  Future<void> addLockShortcut(String targetGroupId) =>
      _fetch(() => _backend.addLockShortcut(targetGroupId));

  Future<void> addCommandShortcut(
    String targetGroupId,
    String label,
    String command,
  ) => _fetch(() => _backend.addCommandShortcut(targetGroupId, label, command));

  /// The installed shortcut providers. A failure yields none: the dialog can
  /// still offer the built-in Lock Screen action, and the error is reported.
  Future<List<ShortcutProvider>> shortcutProviders() async {
    try {
      return await _backend.shortcutProviders();
    } catch (e, st) {
      _report(e, st);
      return const [];
    }
  }

  /// Opens a provider's create screen. The created item arrives later through
  /// the core's model-change event, not as the result of this call.
  Future<void> beginShortcut(String provider, String targetGroupId) async {
    try {
      await _backend.beginShortcut(provider, targetGroupId);
    } catch (e, st) {
      _fail(e, st);
    }
  }

  Future<void> launchShortcut(String id) async {
    try {
      await _backend.launchShortcut(id);
    } catch (e, st) {
      _fail(e, st);
    }
  }

  Future<void> lockScreen() async {
    try {
      await _backend.lockScreen();
    } catch (e, st) {
      _fail(e, st);
    }
  }

  /// A shortcut's stored icon bytes, or `null` when it has none.
  Future<ImageProvider?> itemIcon(String id) async {
    try {
      final bytes = await _backend.itemIcon(id);
      return bytes == null ? null : MemoryImage(Uint8List.fromList(bytes));
    } catch (e, st) {
      _report(e, st);
      return null;
    }
  }

  Future<void> removeItem(String id) => _fetch(() => _backend.removeItem(id));

  Future<void> renameItem(String id, String label) =>
      _fetch(() => _backend.renameItem(id, label));

  Future<void> moveItem(String id, String toGroup, int newOrder) =>
      _fetch(() => _backend.moveItem(id, toGroup, newOrder));

  Future<void> addGroup(String label) => _fetch(() => _backend.addGroup(label));

  Future<void> renameGroup(String id, String label) =>
      _fetch(() => _backend.renameGroup(id, label));

  Future<void> moveGroup(String id, int newOrder) =>
      _fetch(() => _backend.moveGroup(id, newOrder));

  Future<void> setGroupHidden(String id, bool hidden) =>
      _fetch(() => _backend.setGroupHidden(id, hidden));

  Future<void> deleteGroup(String id) => _fetch(() => _backend.deleteGroup(id));

  Future<void> setGroupRole(String id, GroupRole? role) =>
      _fetch(() => _backend.setGroupRole(id, role));

  /// Asks the system to uninstall an app. The core's own list updates when the
  /// package-change receiver fires; a failure is reported through [error].
  Future<void> uninstallApp(String package) async {
    try {
      await _backend.uninstallApp(package);
    } catch (e, st) {
      _fail(e, st);
    }
  }

  /// Opens the system wallpaper chooser. A device with no chooser is a failure
  /// surfaced through [error], never a throw. There is nothing to fetch back:
  /// the window shows the system wallpaper itself, so the change is visible as
  /// soon as the system applies it.
  Future<void> chooseWallpaper() async {
    try {
      await _backend.chooseWallpaper();
    } catch (e, st) {
      _fail(e, st);
    }
  }

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
