import 'dart:async';

import 'package:qlaunch/src/rust/api/api.dart' show ModelSnapshot;
import 'package:qlaunch/src/rust/api/events.dart';
import 'package:qlaunch/src/rust/api/shortcuts.dart';
import 'package:qlaunch/src/rust/model/icon.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/search.dart';
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/rust/usage.dart';
import 'package:qlaunch/src/state/launcher_backend.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

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

  /// The icon returned for an app, whatever size is asked for; the size is in
  /// the recorded call (`iconFor(package,activity,sizePx)`).
  final Map<({String package, String activity}), IconImage> icons = {};

  int get iconCalls => calls.where((c) => c.startsWith('iconFor(')).length;

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
  Future<ModelSnapshot> setGroupHidden(String id, bool hidden) =>
      _snapshotFor('setGroupHidden($id,$hidden)');

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
  Future<ModelSnapshot> factoryReset() => _snapshotFor('factoryReset');

  /// The file name a successful backup answers with.
  String backupName = 'qlaunch-backup-2026-10-21-140530.json';

  /// The backups [listBackups] answers with.
  List<String> backups = const [];

  @override
  Future<String> backupConfig() => _plain('backupConfig', backupName);

  @override
  Future<List<String>> listBackups() => _plain('listBackups', backups);

  @override
  Future<ModelSnapshot> restoreConfig(String fileName) =>
      _snapshotFor('restoreConfig($fileName)');

  @override
  Future<ModelSnapshot> addLockShortcut(String targetGroupId) =>
      _snapshotFor('addLockShortcut($targetGroupId)');

  @override
  Future<ModelSnapshot> addCommandShortcut(
    String targetGroupId,
    String label,
    String command,
  ) => _snapshotFor('addCommandShortcut($targetGroupId,$label,$command)');

  List<ShortcutProvider> shortcutProviderResults = const [];

  @override
  Future<List<ShortcutProvider>> shortcutProviders() =>
      _plain('shortcutProviders', shortcutProviderResults);

  @override
  Future<void> beginShortcut(String provider, String targetGroupId) =>
      _plain('beginShortcut($provider,$targetGroupId)', null);

  @override
  Future<void> launchShortcut(String id) => _plain('launchShortcut($id)', null);

  @override
  Future<void> lockScreen() => _plain('lockScreen', null);

  @override
  Future<List<int>?> itemIcon(String id) =>
      _plain('itemIcon($id)', itemIcons[id]);

  final Map<String, List<int>> itemIcons = {};

  @override
  Future<ModelSnapshot> updateSettings(SettingsPatch patch) {
    settingsPatches.add(patch);
    return _snapshotFor('updateSettings');
  }

  @override
  Future<String?> homeGroupId() => _plain('homeGroupId', current.homeGroupId);

  /// What [searchAllApps] answers for a query. Defaults to no hits.
  List<SearchHit> Function(String query)? searchAllAppsResults;

  /// Every query [searchAllApps] was asked, in order.
  final List<String> searchQueries = [];

  @override
  Future<List<SearchHit>> searchAllApps(String query) {
    searchQueries.add(query);
    return _plain(
      'searchAllApps($query)',
      searchAllAppsResults?.call(query) ?? const <SearchHit>[],
    );
  }

  /// What [frequentApps] answers for a limit. Defaults to no hits.
  List<UsageHit> Function(int limit)? frequentAppsResults;

  /// What [recentApps] answers for a limit. Defaults to no hits.
  List<UsageHit> Function(int limit)? recentAppsResults;

  /// Every usage query asked, in order, e.g. `frequent(24)`.
  final List<String> usageQueries = [];

  @override
  Future<List<UsageHit>> frequentApps(int limit) {
    usageQueries.add('frequent($limit)');
    return _plain(
      'frequentApps($limit)',
      frequentAppsResults?.call(limit) ?? const <UsageHit>[],
    );
  }

  @override
  Future<List<UsageHit>> recentApps(int limit) {
    usageQueries.add('recent($limit)');
    return _plain(
      'recentApps($limit)',
      recentAppsResults?.call(limit) ?? const <UsageHit>[],
    );
  }

  @override
  Future<IconImage?> iconFor(String package, String activity, int sizePx) =>
      _plain<IconImage?>(
        'iconFor($package,$activity,$sizePx)',
        icons[(package: package, activity: activity)],
      );

  @override
  Future<void> launchApp(String package, String activity) async {
    await _plain('launchApp($package,$activity)', null);
    launched.add((package: package, activity: activity));
  }

  @override
  Future<void> uninstallApp(String package) async {
    await _plain('uninstallApp($package)', null);
  }

  @override
  Future<void> chooseWallpaper() async {
    await _plain('chooseWallpaper', null);
  }

  @override
  Stream<CoreEvent> events() {
    calls.add('events');
    return _events.stream;
  }
}

/// [testSettings] with some fields replaced, for tests that vary a setting.
Settings settingsWith({
  int? gridColumnsPortrait,
  int? gridColumnsLandscape,
  LayoutStyle? layout,
  TabVisibility? tabVisibility,
  ScrollFadeStyle? scrollFade,
  bool? continuousScroll,
  int? wallpaperAlpha,
  bool? statusBarVisible,
  int? iconSize,
  int? iconAlpha,
  bool? iconsVisible,
  bool? labelsVisible,
  int? labelSize,
  Typeface? labelTypeface,
  LabelColour? labelColour,
  bool? labelShadow,
  int? labelShadowBlur,
  Orientation? orientation,
}) => Settings(
  gridColumnsPortrait: gridColumnsPortrait ?? testSettings.gridColumnsPortrait,
  gridColumnsLandscape:
      gridColumnsLandscape ?? testSettings.gridColumnsLandscape,
  layout: layout ?? testSettings.layout,
  tabVisibility: tabVisibility ?? testSettings.tabVisibility,
  scrollFade: scrollFade ?? testSettings.scrollFade,
  continuousScroll: continuousScroll ?? testSettings.continuousScroll,
  wallpaperAlpha: wallpaperAlpha ?? testSettings.wallpaperAlpha,
  statusBarVisible: statusBarVisible ?? testSettings.statusBarVisible,
  iconsVisible: iconsVisible ?? testSettings.iconsVisible,
  iconSize: iconSize ?? testSettings.iconSize,
  iconAlpha: iconAlpha ?? testSettings.iconAlpha,
  labelsVisible: labelsVisible ?? testSettings.labelsVisible,
  labelSize: labelSize ?? testSettings.labelSize,
  labelTypeface: labelTypeface ?? testSettings.labelTypeface,
  labelColour: labelColour ?? testSettings.labelColour,
  labelShadow: labelShadow ?? testSettings.labelShadow,
  labelShadowBlur: labelShadowBlur ?? testSettings.labelShadowBlur,
  orientation: orientation ?? testSettings.orientation,
);

/// [snapshot] with its settings replaced.
ModelSnapshot withSettings(ModelSnapshot snapshot, Settings settings) =>
    ModelSnapshot(
      rootGroupId: snapshot.rootGroupId,
      homeGroupId: snapshot.homeGroupId,
      pageIds: snapshot.pageIds,
      groups: snapshot.groups,
      items: snapshot.items,
      settings: settings,
      warnings: snapshot.warnings,
      iconRevisions: snapshot.iconRevisions,
    );

/// Collects what `FlutterError.reportError` is given, for the rest of the test.
///
/// A test's `setUp` cannot install this for a widget test: `testWidgets`
/// installs its own handler after `setUp` has run, so a handler set there is
/// replaced before the body starts.
List<FlutterErrorDetails> captureReports() {
  final reported = <FlutterErrorDetails>[];
  final previous = FlutterError.onError;
  FlutterError.onError = reported.add;
  addTearDown(() => FlutterError.onError = previous);
  return reported;
}
