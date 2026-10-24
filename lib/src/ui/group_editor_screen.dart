import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/group_apps_screen.dart';
import 'package:qlaunch/src/ui/label_style.dart';

/// The label a role shows as in the editor. A page never holds `root` or
/// `hidden`, but the mapping is total so the list cannot draw a blank.
String _roleName(GroupRole role) => switch (role) {
  GroupRole.apps => 'Apps',
  GroupRole.games => 'Games',
  GroupRole.home => 'Home',
  GroupRole.root => 'Root',
  GroupRole.hidden => 'Hidden',
};

/// Lists the tabs in page order, so they can be added and reordered.
///
/// Renaming, changing a role and deleting are Task 8's dialog; the per-group
/// app checklist is Task 9's.
class GroupEditorScreen extends StatelessWidget {
  const GroupEditorScreen({super.key, required this.state});

  static const title = 'Groups';

  final LauncherState state;

  static const addKey = ValueKey('groupEditor.add');
  static const addFieldKey = ValueKey('groupEditor.addField');
  static const addConfirmKey = ValueKey('groupEditor.addConfirm');
  static const addCancelKey = ValueKey('groupEditor.addCancel');

  static const editDialogKey = ValueKey('groupEditor.editDialog');
  static const editLabelFieldKey = ValueKey('groupEditor.editLabel');
  static const editSaveKey = ValueKey('groupEditor.editSave');
  static const editDeleteKey = ValueKey('groupEditor.editDelete');
  static const editCancelKey = ValueKey('groupEditor.editCancel');
  static const confirmDeleteKey = ValueKey('groupEditor.confirmDelete');
  static const selectAppsKey = ValueKey('groupEditor.selectApps');

  /// One per row, keyed by group id so a test can address a tab by name.
  static Key rowKey(String groupId) => ValueKey('groupEditor.row.$groupId');
  static Key upKey(String groupId) => ValueKey('groupEditor.up.$groupId');
  static Key downKey(String groupId) => ValueKey('groupEditor.down.$groupId');
  static Key hiddenKey(String groupId) =>
      ValueKey('groupEditor.hidden.$groupId');

  /// One per role choice, plus the null "None" chip, addressed by name.
  static Key roleKey(GroupRole? role) =>
      ValueKey('groupEditor.role.${role?.name ?? 'none'}');

  Future<void> _add(BuildContext context) async {
    final label = await showDialog<String>(
      context: context,
      builder: (context) => LauncherFontScope(
        settings: state.snapshot?.settings,
        child: const _AddGroupDialog(),
      ),
    );
    if (label == null) return;
    await state.addGroup(label);
  }

  Future<void> _edit(BuildContext context, Group group) async {
    await showDialog<void>(
      context: context,
      builder: (_) => _GroupEditDialog(state: state, group: group),
    );
  }

  /// A group row: its label, its role, a hide switch, and — only for a page —
  /// the reorder arrows. [pageIndex] is -1 for a group that is not shown.
  Widget _row(BuildContext context, Group group, int pageIndex, int pageCount) {
    // The group holding the Hidden role is never a page whatever its flag, so
    // its switch is shown on but disabled.
    final alwaysHidden = group.role == GroupRole.hidden;
    return ListTile(
      key: rowKey(group.id),
      title: Text(group.label),
      subtitle: group.role == null ? null : Text(_roleName(group.role!)),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Switch(
            key: hiddenKey(group.id),
            value: group.hidden || alwaysHidden,
            onChanged: alwaysHidden
                ? null
                : (hidden) => state.setGroupHidden(group.id, hidden),
          ),
          if (pageIndex >= 0) ...[
            IconButton(
              key: upKey(group.id),
              icon: const Icon(Icons.arrow_upward),
              onPressed: pageIndex == 0
                  ? null
                  : () => state.moveGroup(group.id, pageIndex - 1),
            ),
            IconButton(
              key: downKey(group.id),
              icon: const Icon(Icons.arrow_downward),
              onPressed: pageIndex == pageCount - 1
                  ? null
                  : () => state.moveGroup(group.id, pageIndex + 1),
            ),
          ],
        ],
      ),
      onTap: () => _edit(context, group),
    );
  }

  @override
  Widget build(BuildContext context) {
    return LauncherFontScope(
      settings: state.snapshot?.settings,
      child: Scaffold(
        appBar: AppBar(
          title: const Text(title),
          actions: [
            IconButton(
              key: addKey,
              icon: const Icon(Icons.add),
              onPressed: () => _add(context),
            ),
          ],
        ),
        body: ListenableBuilder(
          listenable: state,
          builder: (context, _) {
            // Every top-level group (everything but the root), in `order`, so a
            // hidden group is still listed and can be shown again here. The
            // switch hides a group; the arrows, only on a group that is a page,
            // reorder it.
            final pageIds = state.pageIds;
            final rootId = state.snapshot?.rootGroupId;
            final groups = state.groups
                .where((g) => g.id != rootId && !g.folder)
                .toList()
              ..sort((a, b) => a.order.compareTo(b.order));
            return ListView(
              children: [
                for (final group in groups)
                  _row(
                    context,
                    group,
                    pageIds.indexOf(group.id),
                    pageIds.length,
                  ),
              ],
            );
          },
        ),
      ),
    );
  }
}

/// Renames a tab, changes its role and deletes it.
///
/// §8.4's one-holder-per-role rule is surfaced here: a group holding `Apps`,
/// `Games` or `Hidden` cannot give the role up or be deleted, so its chips and
/// Delete are disabled. Choosing a role another group holds moves it — the core
/// enforces that; the dialog just sends the choice.
class _GroupEditDialog extends StatefulWidget {
  const _GroupEditDialog({required this.state, required this.group});

  final LauncherState state;
  final Group group;

  @override
  State<_GroupEditDialog> createState() => _GroupEditDialogState();
}

class _GroupEditDialogState extends State<_GroupEditDialog> {
  late final TextEditingController _label = TextEditingController(
    text: widget.group.label,
  );
  late GroupRole? _role = widget.group.role;

  /// A role the core will not let go of. `Hidden` is never a page.
  bool get _locked => _role != null && _role != GroupRole.home;

  @override
  void initState() {
    super.initState();
    _label.addListener(_onLabelChanged);
  }

  @override
  void dispose() {
    _label.removeListener(_onLabelChanged);
    _label.dispose();
    super.dispose();
  }

  void _onLabelChanged() => setState(() {});

  Future<void> _setRole(GroupRole? role) async {
    await widget.state.setGroupRole(widget.group.id, role);
    if (mounted) setState(() => _role = role);
  }

  Future<void> _save() async {
    final label = _label.text.trim();
    if (label.isNotEmpty && label != widget.group.label) {
      await widget.state.renameGroup(widget.group.id, label);
    }
    if (mounted) Navigator.of(context).pop();
  }

  Future<void> _confirmDelete() async {
    final navigator = Navigator.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => LauncherFontScope(
        settings: widget.state.snapshot?.settings,
        child: AlertDialog(
          title: Text('Delete ${widget.group.label}?'),
          content: const Text('The apps in this group move to the Apps group.'),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              key: GroupEditorScreen.confirmDeleteKey,
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text('Delete'),
            ),
          ],
        ),
      ),
    );
    if (confirmed != true) return;
    await widget.state.deleteGroup(widget.group.id);
    if (!mounted) return;
    // Close the edit dialog the confirmation was opened from.
    navigator.pop();
  }

  @override
  Widget build(BuildContext context) {
    return LauncherFontScope(
      settings: widget.state.snapshot?.settings,
      child: AlertDialog(
        key: GroupEditorScreen.editDialogKey,
        title: Text(widget.group.label),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            TextField(
              key: GroupEditorScreen.editLabelFieldKey,
              controller: _label,
              decoration: const InputDecoration(labelText: 'Name'),
            ),
            const SizedBox(height: 16),
            Text('Role', style: Theme.of(context).textTheme.labelLarge),
            const SizedBox(height: 4),
            Wrap(
              spacing: 8,
              children: [
                for (final role in const [
                  GroupRole.home,
                  GroupRole.apps,
                  GroupRole.games,
                ])
                  ChoiceChip(
                    key: GroupEditorScreen.roleKey(role),
                    label: Text(_roleName(role)),
                    selected: _role == role,
                    onSelected: _locked ? null : (_) => _setRole(role),
                  ),
                ChoiceChip(
                  key: GroupEditorScreen.roleKey(null),
                  label: const Text('None'),
                  selected: _role == null,
                  onSelected: _role == GroupRole.home
                      ? (_) => _setRole(null)
                      : null,
                ),
              ],
            ),
            ListTile(
              key: GroupEditorScreen.selectAppsKey,
              contentPadding: EdgeInsets.zero,
              leading: const Icon(Icons.checklist),
              title: const Text('Choose apps'),
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute<void>(
                  builder: (_) => GroupAppsScreen(
                    state: widget.state,
                    groupId: widget.group.id,
                  ),
                ),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            key: GroupEditorScreen.editDeleteKey,
            onPressed: _role == null ? _confirmDelete : null,
            child: const Text('Delete'),
          ),
          TextButton(
            key: GroupEditorScreen.editCancelKey,
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: GroupEditorScreen.editSaveKey,
            onPressed: _label.text.trim().isEmpty ? null : _save,
            child: const Text('Save'),
          ),
        ],
      ),
    );
  }
}

/// Asks for a new group's label. Returns the trimmed label, or null on cancel.
class _AddGroupDialog extends StatefulWidget {
  const _AddGroupDialog();

  @override
  State<_AddGroupDialog> createState() => _AddGroupDialogState();
}

class _AddGroupDialogState extends State<_AddGroupDialog> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final label = _controller.text.trim();
    return AlertDialog(
      title: const Text('Add group'),
      content: TextField(
        key: GroupEditorScreen.addFieldKey,
        controller: _controller,
        autofocus: true,
        decoration: const InputDecoration(labelText: 'Name'),
        onChanged: (_) => setState(() {}),
      ),
      actions: [
        TextButton(
          key: GroupEditorScreen.addCancelKey,
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: GroupEditorScreen.addConfirmKey,
          onPressed: label.isEmpty
              ? null
              : () => Navigator.of(context).pop(label),
          child: const Text('Add'),
        ),
      ],
    );
  }
}
