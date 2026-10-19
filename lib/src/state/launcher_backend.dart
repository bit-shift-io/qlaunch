import 'dart:typed_data';

import 'package:qlaunch/src/rust/api/api.dart' as rust;
import 'package:qlaunch/src/rust/api/api.dart' show ModelSnapshot;
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/rust/settings.dart';

/// The whole bridge surface, with positional parameters. Only `LauncherState`
/// talks to this.
abstract class LauncherBackend {
  Future<ModelSnapshot> init({String? configRootOverride});
  Future<ModelSnapshot> snapshot();
  Future<ModelSnapshot> refreshApps();
  Future<ModelSnapshot> addGroup(String label);
  Future<ModelSnapshot> renameGroup(String id, String label);
  Future<ModelSnapshot> moveGroup(String id, int newOrder);
  Future<ModelSnapshot> setGroupRole(String id, GroupRole? role);
  Future<ModelSnapshot> deleteGroup(String id);
  Future<ModelSnapshot> moveItem(String id, String toGroup, int newOrder);
  Future<ModelSnapshot> renameItem(String id, String label);
  Future<ModelSnapshot> setItemIcon(String id, List<int> png);
  Future<ModelSnapshot> removeItem(String id);
  Future<ModelSnapshot> sortGroup(String id);
  Future<ModelSnapshot> updateSettings(SettingsPatch patch);
  Future<String?> homeGroupId();
  Future<List<SearchHit>> searchAllApps(String query);
  Future<Uint8List?> iconFor(String package, String activity, int sizePx);
  Future<void> launchApp(String package, String activity);
  Future<void> uninstallApp(String package);
  Stream<CoreEvent> events();
}

/// Forwards one-for-one to the generated bindings. No logic lives here.
class RustBackend implements LauncherBackend {
  const RustBackend();

  @override
  Future<ModelSnapshot> init({String? configRootOverride}) =>
      rust.init(configRootOverride: configRootOverride);

  @override
  Future<ModelSnapshot> snapshot() => rust.snapshot();

  @override
  Future<ModelSnapshot> refreshApps() => rust.refreshApps();

  @override
  Future<ModelSnapshot> addGroup(String label) => rust.addGroup(label: label);

  @override
  Future<ModelSnapshot> renameGroup(String id, String label) =>
      rust.renameGroup(id: id, label: label);

  @override
  Future<ModelSnapshot> moveGroup(String id, int newOrder) =>
      rust.moveGroup(id: id, newOrder: newOrder);

  @override
  Future<ModelSnapshot> setGroupRole(String id, GroupRole? role) =>
      rust.setGroupRole(id: id, role: role);

  @override
  Future<ModelSnapshot> deleteGroup(String id) => rust.deleteGroup(id: id);

  @override
  Future<ModelSnapshot> moveItem(String id, String toGroup, int newOrder) =>
      rust.moveItem(id: id, toGroup: toGroup, newOrder: newOrder);

  @override
  Future<ModelSnapshot> renameItem(String id, String label) =>
      rust.renameItem(id: id, label: label);

  @override
  Future<ModelSnapshot> setItemIcon(String id, List<int> png) =>
      rust.setItemIcon(id: id, png: png);

  @override
  Future<ModelSnapshot> removeItem(String id) => rust.removeItem(id: id);

  @override
  Future<ModelSnapshot> sortGroup(String id) => rust.sortGroup(id: id);

  @override
  Future<ModelSnapshot> updateSettings(SettingsPatch patch) =>
      rust.updateSettings(patch: patch);

  @override
  Future<String?> homeGroupId() => rust.homeGroupId();

  @override
  Future<List<SearchHit>> searchAllApps(String query) =>
      rust.searchAllApps(query: query);

  @override
  Future<Uint8List?> iconFor(String package, String activity, int sizePx) =>
      rust.iconFor(package: package, activity: activity, sizePx: sizePx);

  @override
  Future<void> launchApp(String package, String activity) =>
      rust.launchApp(package: package, activity: activity);

  @override
  Future<void> uninstallApp(String package) =>
      rust.uninstallApp(packageName: package);

  @override
  Stream<CoreEvent> events() => rust.events();
}
