import 'package:qlaunch/src/rust/api/api.dart' as rust;
import 'package:qlaunch/src/rust/api/api.dart' show ModelSnapshot;
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/api/shortcuts.dart';
import 'package:qlaunch/src/rust/model/icon.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/rust/usage.dart';

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
  Future<ModelSnapshot> setGroupHidden(String id, bool hidden);
  Future<ModelSnapshot> deleteGroup(String id);
  Future<ModelSnapshot> moveItem(String id, String toGroup, int newOrder);
  Future<ModelSnapshot> renameItem(String id, String label);
  Future<ModelSnapshot> setItemIcon(String id, List<int> png);
  Future<ModelSnapshot> removeItem(String id);
  Future<ModelSnapshot> sortGroup(String id);
  Future<ModelSnapshot> factoryReset();
  Future<String> backupConfig();
  Future<List<String>> listBackups();
  Future<ModelSnapshot> restoreConfig(String fileName);
  Future<ModelSnapshot> addLockShortcut(String targetGroupId);
  Future<ModelSnapshot> addNotificationsShortcut(String targetGroupId);
  Future<ModelSnapshot> addCommandShortcut(
    String targetGroupId,
    String label,
    String command,
  );
  Future<List<ShortcutProvider>> shortcutProviders();
  Future<void> beginShortcut(String provider, String targetGroupId);
  Future<void> launchShortcut(String id);
  Future<void> lockScreen();
  Future<void> openNotifications();
  Future<List<int>?> itemIcon(String id);
  Future<ModelSnapshot> updateSettings(SettingsPatch patch);
  Future<String?> homeGroupId();
  Future<List<SearchHit>> searchAllApps(String query);
  Future<List<UsageHit>> frequentApps(int limit);
  Future<List<UsageHit>> recentApps(int limit);
  Future<IconImage?> iconFor(String package, String activity, int sizePx);
  Future<void> launchApp(String package, String activity);
  Future<void> uninstallApp(String package);
  Future<void> chooseWallpaper();
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
  Future<ModelSnapshot> setGroupHidden(String id, bool hidden) =>
      rust.setGroupHidden(id: id, hidden: hidden);

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
  Future<ModelSnapshot> factoryReset() => rust.factoryReset();

  @override
  Future<String> backupConfig() => rust.backupConfig();

  @override
  Future<List<String>> listBackups() => rust.listBackups();

  @override
  Future<ModelSnapshot> restoreConfig(String fileName) =>
      rust.restoreConfig(fileName: fileName);

  @override
  Future<ModelSnapshot> addLockShortcut(String targetGroupId) =>
      rust.addLockShortcut(targetGroupId: targetGroupId);

  @override
  Future<ModelSnapshot> addNotificationsShortcut(String targetGroupId) =>
      rust.addNotificationsShortcut(targetGroupId: targetGroupId);

  @override
  Future<ModelSnapshot> addCommandShortcut(
    String targetGroupId,
    String label,
    String command,
  ) => rust.addCommandShortcut(
    targetGroupId: targetGroupId,
    label: label,
    command: command,
  );

  @override
  Future<List<ShortcutProvider>> shortcutProviders() =>
      rust.shortcutProviders();

  @override
  Future<void> beginShortcut(String provider, String targetGroupId) =>
      rust.beginShortcut(provider: provider, targetGroupId: targetGroupId);

  @override
  Future<void> launchShortcut(String id) => rust.launchShortcut(id: id);

  @override
  Future<void> lockScreen() => rust.lockScreen();

  @override
  Future<void> openNotifications() => rust.openNotifications();

  @override
  Future<List<int>?> itemIcon(String id) => rust.itemIcon(id: id);

  @override
  Future<ModelSnapshot> updateSettings(SettingsPatch patch) =>
      rust.updateSettings(patch: patch);

  @override
  Future<String?> homeGroupId() => rust.homeGroupId();

  @override
  Future<List<SearchHit>> searchAllApps(String query) =>
      rust.searchAllApps(query: query);

  @override
  Future<List<UsageHit>> frequentApps(int limit) =>
      rust.frequentApps(limit: limit);

  @override
  Future<List<UsageHit>> recentApps(int limit) => rust.recentApps(limit: limit);

  @override
  Future<IconImage?> iconFor(String package, String activity, int sizePx) =>
      rust.iconFor(package: package, activity: activity, sizePx: sizePx);

  @override
  Future<void> launchApp(String package, String activity) =>
      rust.launchApp(package: package, activity: activity);

  @override
  Future<void> uninstallApp(String package) =>
      rust.uninstallApp(packageName: package);

  @override
  Future<void> chooseWallpaper() => rust.chooseWallpaper();

  @override
  Stream<CoreEvent> events() => rust.events();
}
