import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/label_style.dart';

/// Creates a folder, or renames, moves and deletes an existing one.
///
/// A folder's tile chooses which page it appears on; the folder group itself is
/// never a page. The group list therefore excludes the root, other folders and
/// the Hidden role, none of which can hold a tile.
class FolderDialog extends StatefulWidget {
  const FolderDialog._({
    required this.state,
    this.containingGroupId,
    this.folderGroup,
  });

  factory FolderDialog.create({
    required LauncherState state,
    required String containingGroupId,
  }) => FolderDialog._(state: state, containingGroupId: containingGroupId);

  factory FolderDialog.edit({
    required LauncherState state,
    required Group folderGroup,
  }) => FolderDialog._(state: state, folderGroup: folderGroup);

  final LauncherState state;
  final String? containingGroupId;
  final Group? folderGroup;

  bool get isEdit => folderGroup != null;

  static const nameKey = ValueKey('folderDialog.name');
  static const groupKey = ValueKey('folderDialog.group');
  static const saveKey = ValueKey('folderDialog.save');
  static const cancelKey = ValueKey('folderDialog.cancel');
  static const deleteKey = ValueKey('folderDialog.delete');

  static Key groupOptionKey(String groupId) =>
      ValueKey('folderDialog.group.$groupId');

  @override
  State<FolderDialog> createState() => _FolderDialogState();
}

class _FolderDialogState extends State<FolderDialog> {
  late final TextEditingController _name;
  late String _groupId;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    final folder = widget.folderGroup;
    _name = TextEditingController(text: folder?.label ?? 'Folder');
    final candidates = _candidates();
    final preferred = folder == null
        ? widget.containingGroupId
        : widget.state.folderTile(folder.id)?.groupId;
    if (folder != null && preferred != null && preferred.isNotEmpty) {
      // The current group may be excluded (the Hidden group); the dropdown
      // adds it back so the initial value stays valid.
      _groupId = preferred;
    } else {
      _groupId = candidates.any((g) => g.id == preferred)
          ? preferred!
          : (candidates.isEmpty ? '' : candidates.first.id);
    }
  }

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  List<Group> _candidates() {
    final rootId = widget.state.snapshot?.rootGroupId;
    return widget.state.groups
        .where(
          (g) =>
              g.id != rootId &&
              !g.folder &&
              g.role != GroupRole.hidden,
        )
        .toList();
  }

  /// The candidates, plus the folder's current group when that group is not
  /// otherwise offered, so the dropdown always has a valid initial value.
  List<Group> _destinations() {
    final groups = _candidates();
    if (!groups.any((g) => g.id == _groupId)) {
      final current = widget.state.groupById(_groupId);
      if (current != null) groups.add(current);
    }
    return groups;
  }

  Future<void> _save() async {
    final label = _name.text.trim();
    if (_busy || label.isEmpty || _groupId.isEmpty) return;
    setState(() => _busy = true);
    final folder = widget.folderGroup;
    if (folder == null) {
      await widget.state.addFolder(_groupId, label);
    } else {
      if (label != folder.label) {
        await widget.state.renameGroup(folder.id, label);
      }
      final tile = widget.state.folderTile(folder.id);
      if (tile != null && tile.groupId != _groupId) {
        await widget.state.moveItem(
          tile.id,
          _groupId,
          widget.state.itemsIn(_groupId).length,
        );
      }
    }
    if (mounted) Navigator.of(context).pop();
  }

  Future<void> _delete() async {
    final folder = widget.folderGroup;
    if (_busy || folder == null) return;
    setState(() => _busy = true);
    await widget.state.deleteFolder(folder.id);
    if (mounted) Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final destinations = _destinations();
    final canSave =
        !_busy && _name.text.trim().isNotEmpty && _groupId.isNotEmpty;

    return LauncherFontScope(
      settings: widget.state.snapshot?.settings,
      child: AlertDialog(
        title: Text(widget.folderGroup?.label ?? 'New Folder'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              key: FolderDialog.nameKey,
              controller: _name,
              onChanged: (_) => setState(() {}),
              decoration: const InputDecoration(labelText: 'Name'),
            ),
            const SizedBox(height: 16),
            DropdownButtonFormField<String>(
              key: FolderDialog.groupKey,
              initialValue: _groupId.isEmpty ? null : _groupId,
              decoration: const InputDecoration(labelText: 'Group'),
              items: [
                for (final group in destinations)
                  DropdownMenuItem(
                    key: FolderDialog.groupOptionKey(group.id),
                    value: group.id,
                    child: Text(group.label),
                  ),
              ],
              onChanged: (value) {
                if (value != null) setState(() => _groupId = value);
              },
            ),
          ],
        ),
        actions: [
          if (widget.isEdit)
            TextButton(
              key: FolderDialog.deleteKey,
              onPressed: _busy ? null : _delete,
              child: const Text('Delete'),
            ),
          TextButton(
            key: FolderDialog.cancelKey,
            onPressed: _busy ? null : () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: FolderDialog.saveKey,
            onPressed: canSave ? _save : null,
            child: const Text('Save'),
          ),
        ],
      ),
    );
  }
}
