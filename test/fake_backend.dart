import 'dart:async';
import 'dart:typed_data';

import 'package:qlaunch/src/rust/api/api.dart';
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/state/launcher_backend.dart';

const _defaultSettings = Settings(
  gridColumnsPortrait: 4,
  gridColumnsLandscape: 6,
  layout: LayoutStyle.tiles,
  tabVisibility: TabVisibility.always,
  wallpaperAlpha: 255,
  statusBarVisible: true,
  iconsVisible: true,
  iconSize: 48,
  iconAlpha: 255,
  labelsVisible: true,
  labelSize: 12,
  labelTypeface: Typeface.normal,
  labelColour: LabelColour.white,
  labelShadow: true,
  orientation: Orientation.auto,
);

/// A hand-written [LauncherBackend] for tests. It holds one snapshot, which
/// every call returns; change [current] to make a re-fetch see new data.
class FakeBackend implements LauncherBackend {
  FakeBackend({ModelSnapshot? snapshot})
    : current =
          snapshot ??
          const ModelSnapshot(
            rootGroupId: 'root',
            pageIds: [],
            groups: [],
            items: [],
            settings: _defaultSettings,
            warnings: [],
          );

  ModelSnapshot current;

  /// Backend calls in the order they were made, by method name.
  final List<String> calls = [];

  int get snapshotCalls => calls.where((c) => c == 'snapshot').length;

  final List<({String package, String activity})> launched = [];

  String? lastConfigRootOverride;

  /// When set, every future-returning call throws this after being recorded.
  Object? failure;

  /// When set, `snapshot()` waits on this before answering, so a test can
  /// hold a re-fetch in flight.
  Future<void>? snapshotGate;

  final StreamController<CoreEvent> _events = StreamController.broadcast();

  void emit(CoreEvent event) => _events.add(event);

  Future<void> close() => _events.close();

  Future<void> _record(String name) async {
    calls.add(name);
    final gate = snapshotGate;
    if (name == 'snapshot' && gate != null) await gate;
    final f = failure;
    if (f != null) throw f;
  }

  Future<ModelSnapshot> _snapshotFor(String name) async {
    await _record(name);
    return current;
  }

  @override
  Future<ModelSnapshot> init({String? configRootOverride}) {
    lastConfigRootOverride = configRootOverride;
    return _snapshotFor('init');
  }

  @override
  Future<ModelSnapshot> snapshot() => _snapshotFor('snapshot');

  @override
  Future<ModelSnapshot> refreshApps() => _snapshotFor('refreshApps');

  @override
  Future<ModelSnapshot> addGroup(String label) => _snapshotFor('addGroup');

  @override
  Future<ModelSnapshot> renameGroup(String id, String label) =>
      _snapshotFor('renameGroup');

  @override
  Future<ModelSnapshot> moveGroup(String id, int newOrder) =>
      _snapshotFor('moveGroup');

  @override
  Future<ModelSnapshot> setGroupRole(String id, GroupRole? role) =>
      _snapshotFor('setGroupRole');

  @override
  Future<ModelSnapshot> deleteGroup(String id) => _snapshotFor('deleteGroup');

  @override
  Future<ModelSnapshot> moveItem(String id, String toGroup, int newOrder) =>
      _snapshotFor('moveItem');

  @override
  Future<ModelSnapshot> renameItem(String id, String label) =>
      _snapshotFor('renameItem');

  @override
  Future<ModelSnapshot> setItemIcon(String id, List<int> png) =>
      _snapshotFor('setItemIcon');

  @override
  Future<ModelSnapshot> removeItem(String id) => _snapshotFor('removeItem');

  @override
  Future<ModelSnapshot> sortGroup(String id) => _snapshotFor('sortGroup');

  @override
  Future<ModelSnapshot> updateSettings(SettingsPatch patch) =>
      _snapshotFor('updateSettings');

  @override
  Future<String?> homeGroupId() async {
    await _record('homeGroupId');
    return current.homeGroupId;
  }

  @override
  Future<List<SearchHit>> searchAllApps(String query) async {
    await _record('searchAllApps');
    return const [];
  }

  @override
  Future<Uint8List?> iconFor(
    String package,
    String activity,
    int sizePx,
  ) async {
    await _record('iconFor');
    return null;
  }

  @override
  Future<void> launchApp(String package, String activity) async {
    await _record('launchApp');
    launched.add((package: package, activity: activity));
  }

  @override
  Stream<CoreEvent> events() {
    calls.add('events');
    return _events.stream;
  }
}
