import 'dart:async';
import 'dart:typed_data';

import 'package:qlaunch/src/rust/api/api.dart' show ModelSnapshot;
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/state/launcher_backend.dart';

import 'snapshot_builders.dart';

/// A hand-written [LauncherBackend] for tests. Every snapshot-returning call
/// answers with [current] as it was *when the call was made*, like the real
/// core reading its state at the start of a request.
class FakeBackend implements LauncherBackend {
  /// With [hangOnCancel], cancelling the `events()` subscription never
  /// completes, as with the real stream on a quiet machine.
  FakeBackend({ModelSnapshot? snapshot, bool hangOnCancel = false})
    : current = snapshot ?? snapshotOf(),
      _events = hangOnCancel
          ? StreamController<CoreEvent>(
              onCancel: () => Completer<void>().future,
            )
          : StreamController<CoreEvent>.broadcast();

  ModelSnapshot current;

  /// Backend calls in the order they were made. Mutations carry their
  /// arguments, e.g. `moveItem(a,games,3)`; the rest are bare names.
  final List<String> calls = [];

  int get snapshotCalls => calls.where((c) => c == 'snapshot').length;

  final List<({String package, String activity})> launched = [];

  final List<SettingsPatch> settingsPatches = [];

  String? lastConfigRootOverride;

  /// When set, every future-returning call throws this, as the call is made.
  Object? failure;

  /// While true, each snapshot-returning call parks on a new completer in
  /// [held] until the test completes it, so tests choose the order in which
  /// in-flight calls finish.
  bool holdSnapshots = false;
  final List<Completer<void>> held = [];

  final StreamController<CoreEvent> _events;

  void emit(CoreEvent event) => _events.add(event);

  void emitError(Object error) => _events.addError(error);

  Future<void> close() => _events.close();

  Future<ModelSnapshot> _snapshotFor(String entry) async {
    calls.add(entry);
    final result = current;
    final f = failure;
    if (holdSnapshots) {
      final gate = Completer<void>();
      held.add(gate);
      await gate.future;
    }
    if (f != null) throw f;
    return result;
  }

  Future<T> _plain<T>(String entry, T result) async {
    calls.add(entry);
    final f = failure;
    if (f != null) throw f;
    return result;
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
  Future<ModelSnapshot> addGroup(String label) =>
      _snapshotFor('addGroup($label)');

  @override
  Future<ModelSnapshot> renameGroup(String id, String label) =>
      _snapshotFor('renameGroup($id,$label)');

  @override
  Future<ModelSnapshot> moveGroup(String id, int newOrder) =>
      _snapshotFor('moveGroup($id,$newOrder)');

  @override
  Future<ModelSnapshot> setGroupRole(String id, GroupRole? role) =>
      _snapshotFor('setGroupRole($id,${role?.name})');

  @override
  Future<ModelSnapshot> deleteGroup(String id) =>
      _snapshotFor('deleteGroup($id)');

  @override
  Future<ModelSnapshot> moveItem(String id, String toGroup, int newOrder) =>
      _snapshotFor('moveItem($id,$toGroup,$newOrder)');

  @override
  Future<ModelSnapshot> renameItem(String id, String label) =>
      _snapshotFor('renameItem($id,$label)');

  @override
  Future<ModelSnapshot> setItemIcon(String id, List<int> png) =>
      _snapshotFor('setItemIcon($id,${png.length} bytes)');

  @override
  Future<ModelSnapshot> removeItem(String id) =>
      _snapshotFor('removeItem($id)');

  @override
  Future<ModelSnapshot> sortGroup(String id) => _snapshotFor('sortGroup($id)');

  @override
  Future<ModelSnapshot> updateSettings(SettingsPatch patch) {
    settingsPatches.add(patch);
    return _snapshotFor('updateSettings');
  }

  @override
  Future<String?> homeGroupId() => _plain('homeGroupId', current.homeGroupId);

  @override
  Future<List<SearchHit>> searchAllApps(String query) =>
      _plain('searchAllApps($query)', const <SearchHit>[]);

  @override
  Future<Uint8List?> iconFor(String package, String activity, int sizePx) =>
      _plain<Uint8List?>('iconFor($package,$activity,$sizePx)', null);

  @override
  Future<void> launchApp(String package, String activity) async {
    await _plain('launchApp($package,$activity)', null);
    launched.add((package: package, activity: activity));
  }

  @override
  Stream<CoreEvent> events() {
    calls.add('events');
    return _events.stream;
  }
}
