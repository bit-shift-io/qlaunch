import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/state/launcher_state.dart';

/// Every app, ticked when it belongs to [groupId].
///
/// Ticking moves the app into the group; unticking moves it to the Apps group.
/// In the Apps group itself, unticking is disabled: there is nowhere for the
/// app to go — every app is in Apps or can be.
class GroupAppsScreen extends StatelessWidget {
  const GroupAppsScreen({
    super.key,
    required this.state,
    required this.groupId,
  });

  final LauncherState state;
  final String groupId;

  static Key tileKey(String itemId) => ValueKey('groupApps.$itemId');

  String _title() {
    for (final group in state.groups) {
      if (group.id == groupId) return group.label;
    }
    return '';
  }

  Future<void> _toggle(Item item, bool checked) async {
    // A CheckboxListTile only calls this with the toggled value, so a tick
    // always means "not a member" and an untick always means "member": the
    // move is never a same-group no-op.
    if (checked) {
      await state.moveItem(item.id, groupId, state.itemsIn(groupId).length);
      return;
    }
    final apps = state.appsGroup;
    if (apps == null) return;
    await state.moveItem(item.id, apps.id, state.itemsIn(apps.id).length);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(_title())),
      body: ListenableBuilder(
        listenable: state,
        builder: (context, _) {
          final appsId = state.appsGroup?.id;
          final items =
              [
                for (final item in state.snapshot?.items ?? const <Item>[])
                  if (item.app != null && !item.uninstalled) item,
              ]..sort(
                (a, b) => a.label.toLowerCase().compareTo(b.label.toLowerCase()),
              );
          return ListView.builder(
            itemCount: items.length,
            itemBuilder: (context, index) {
              final item = items[index];
              return CheckboxListTile(
                key: tileKey(item.id),
                title: Text(item.label),
                value: item.groupId == groupId,
                onChanged: groupId == appsId
                    ? null
                    : (checked) => _toggle(item, checked ?? false),
              );
            },
          );
        },
      ),
    );
  }
}
