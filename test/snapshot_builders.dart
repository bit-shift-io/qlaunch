import 'package:qlaunch/src/rust/api/api.dart' show ModelSnapshot;
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/rust/settings.dart';

const testSettings = Settings(
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

Group makeGroup(String id, {GroupRole? role, int order = 0}) =>
    Group(id: id, label: id, role: role, order: order);

Item makeItem(String id, String groupId, {int order = 0}) => Item(
  id: id,
  groupId: groupId,
  label: id,
  labelOverridden: false,
  uninstalled: false,
  order: order,
);

/// `homeGroupId` is optional on [ModelSnapshot], so a snapshot built without
/// it silently has no home group: pass it wherever a test means to have one.
ModelSnapshot snapshotOf({
  String rootGroupId = 'root',
  String? homeGroupId,
  List<String> pageIds = const [],
  List<Group> groups = const [],
  List<Item> items = const [],
}) => ModelSnapshot(
  rootGroupId: rootGroupId,
  homeGroupId: homeGroupId,
  pageIds: pageIds,
  groups: groups,
  items: items,
  settings: testSettings,
  warnings: const [],
);
