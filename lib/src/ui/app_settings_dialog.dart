import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/label_style.dart';

/// The per-app popup: rename it, move it to another group, or uninstall it.
///
/// The group list excludes the root group, which cannot hold items, and
/// includes the Hidden group, so an app can be hidden from a tab here.
class AppSettingsDialog extends StatefulWidget {
  const AppSettingsDialog({super.key, required this.state, required this.item});

  final LauncherState state;
  final Item item;

  static const labelFieldKey = ValueKey('appSettings.label');
  static const saveKey = ValueKey('appSettings.save');
  static const cancelKey = ValueKey('appSettings.cancel');
  static const uninstallKey = ValueKey('appSettings.uninstall');
  static const removeKey = ValueKey('appSettings.remove');
  static const groupKey = ValueKey('appSettings.group');

  static Key groupOptionKey(String groupId) =>
      ValueKey('appSettings.group.$groupId');

  @override
  State<AppSettingsDialog> createState() => _AppSettingsDialogState();
}

class _AppSettingsDialogState extends State<AppSettingsDialog> {
  late final TextEditingController _label;
  late String _groupId;

  @override
  void initState() {
    super.initState();
    _label = TextEditingController(text: widget.item.label);
    _groupId = widget.item.groupId;
  }

  @override
  void dispose() {
    _label.dispose();
    super.dispose();
  }

  void _close() {
    final navigator = Navigator.of(context);
    if (navigator.canPop()) navigator.pop();
  }

  Future<void> _save() async {
    final label = _label.text.trim();
    if (label != widget.item.label) {
      await widget.state.renameItem(widget.item.id, label);
    }
    if (_groupId != widget.item.groupId) {
      await widget.state.moveItem(
        widget.item.id,
        _groupId,
        widget.state.itemsIn(_groupId).length,
      );
    }
    if (mounted) _close();
  }

  Future<void> _uninstall() async {
    final app = widget.item.app;
    if (app == null) return;
    await widget.state.uninstallApp(app.package);
    if (mounted) _close();
  }

  Future<void> _remove() async {
    await widget.state.removeItem(widget.item.id);
    if (mounted) _close();
  }

  @override
  Widget build(BuildContext context) {
    final item = widget.item;
    final rootId = widget.state.snapshot?.rootGroupId;
    final destinations = widget.state.groups
        .where((g) => g.id != rootId)
        .toList();
    final canUninstall = item.app != null && !item.uninstalled;
    final canSave = _label.text.trim().isNotEmpty;

    return LauncherFontScope(
      settings: widget.state.snapshot?.settings,
      child: AlertDialog(
        title: Text(item.label),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              key: AppSettingsDialog.labelFieldKey,
              controller: _label,
              onChanged: (_) => setState(() {}),
              decoration: const InputDecoration(labelText: 'Name'),
            ),
            if (item.app != null) ...[
              const SizedBox(height: 12),
              Text(
                item.app!.package,
                style: Theme.of(context).textTheme.bodySmall,
              ),
              if (item.uninstalled)
                Text(
                  'Uninstalled',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
            ],
            const SizedBox(height: 16),
            DropdownButtonFormField<String>(
              key: AppSettingsDialog.groupKey,
              initialValue: _groupId,
              decoration: const InputDecoration(labelText: 'Group'),
              items: [
                for (final group in destinations)
                  DropdownMenuItem(
                    key: AppSettingsDialog.groupOptionKey(group.id),
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
          if (canUninstall)
            TextButton(
              key: AppSettingsDialog.uninstallKey,
              onPressed: _uninstall,
              child: const Text('Uninstall'),
            ),
          if (item.shortcut != null)
            TextButton(
              key: AppSettingsDialog.removeKey,
              onPressed: _remove,
              child: const Text('Remove'),
            ),
          TextButton(
            key: AppSettingsDialog.cancelKey,
            onPressed: _close,
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: AppSettingsDialog.saveKey,
            onPressed: canSave ? _save : null,
            child: const Text('Save'),
          ),
        ],
      ),
    );
  }
}
