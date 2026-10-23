import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/api/shortcuts.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/group_editor_screen.dart';
import 'package:qlaunch/src/ui/settings_screen.dart';

/// What a long press on blank launcher space offers, and the shortcut picker
/// it can open.
class LauncherMenu {
  static const settingsKey = ValueKey('launcherMenu.settings');
  static const addShortcutKey = ValueKey('launcherMenu.addShortcut');
  static const editGroupsKey = ValueKey('launcherMenu.editGroups');
  static const wallpaperKey = ValueKey('launcherMenu.wallpaper');

  static Future<void> show(
    BuildContext context,
    LauncherState state,
    String groupId,
  ) async {
    final action = await showDialog<_MenuAction>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Launcher'),
        contentPadding: const EdgeInsets.symmetric(vertical: 8),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              key: settingsKey,
              leading: const Icon(Icons.settings),
              title: const Text('Launcher Settings'),
              onTap: () => Navigator.of(context).pop(_MenuAction.settings),
            ),
            ListTile(
              key: addShortcutKey,
              leading: const Icon(Icons.add_link),
              title: const Text('Add Shortcut'),
              onTap: () => Navigator.of(context).pop(_MenuAction.shortcut),
            ),
            ListTile(
              key: editGroupsKey,
              leading: const Icon(Icons.dashboard_customize),
              title: const Text('Edit Groups'),
              onTap: () => Navigator.of(context).pop(_MenuAction.groups),
            ),
            ListTile(
              key: wallpaperKey,
              leading: const Icon(Icons.wallpaper),
              title: const Text('Choose Wallpaper'),
              onTap: () => Navigator.of(context).pop(_MenuAction.wallpaper),
            ),
          ],
        ),
      ),
    );
    if (!context.mounted || action == null) return;
    switch (action) {
      case _MenuAction.settings:
        await Navigator.of(context).push(
          MaterialPageRoute<void>(
            builder: (context) => SettingsScreen(state: state),
          ),
        );
      case _MenuAction.shortcut:
        await showDialog<void>(
          context: context,
          builder: (context) =>
              AddShortcutDialog(state: state, targetGroupId: groupId),
        );
      case _MenuAction.groups:
        await Navigator.of(context).push(
          MaterialPageRoute<void>(
            builder: (context) => GroupEditorScreen(state: state),
          ),
        );
      case _MenuAction.wallpaper:
        await state.chooseWallpaper();
    }
  }
}

enum _MenuAction { settings, shortcut, groups, wallpaper }

/// The list of ways to create a shortcut: the built-in lock action, a typed
/// command on Linux, and whichever providers Android reports.
class AddShortcutDialog extends StatefulWidget {
  const AddShortcutDialog({
    super.key,
    required this.state,
    required this.targetGroupId,
  });

  final LauncherState state;
  final String targetGroupId;

  static const lockKey = ValueKey('addShortcut.lock');
  static const notificationsKey = ValueKey('addShortcut.notifications');
  static const commandKey = ValueKey('addShortcut.command');
  static const commandLabelKey = ValueKey('addShortcut.commandLabel');
  static const commandTextKey = ValueKey('addShortcut.commandText');
  static const commandSaveKey = ValueKey('addShortcut.commandSave');

  static Key providerKey(String id) => ValueKey('addShortcut.provider.$id');

  @override
  State<AddShortcutDialog> createState() => _AddShortcutDialogState();
}

class _AddShortcutDialogState extends State<AddShortcutDialog> {
  late final Future<List<ShortcutProvider>> _providers;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _providers = widget.state.shortcutProviders();
  }

  Future<void> _run(Future<void> Function() action) async {
    if (_busy) return;
    setState(() => _busy = true);
    await action();
    if (mounted) Navigator.of(context).pop();
  }

  Future<void> _addCommand() async {
    final result = await showDialog<({String label, String command})>(
      context: context,
      builder: (context) => const _CommandShortcutDialog(),
    );
    if (result == null || !mounted) return;
    await _run(
      () => widget.state.addCommandShortcut(
        widget.targetGroupId,
        result.label,
        result.command,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add Shortcut'),
      content: FutureBuilder<List<ShortcutProvider>>(
        future: _providers,
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const SizedBox(
              height: 64,
              child: Center(child: CircularProgressIndicator()),
            );
          }
          final providers = snapshot.data ?? const <ShortcutProvider>[];
          return SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                ListTile(
                  key: AddShortcutDialog.lockKey,
                  leading: const Icon(Icons.lock_outline),
                  title: const Text('Lock Screen'),
                  enabled: !_busy,
                  onTap: () => _run(
                    () => widget.state.addLockShortcut(widget.targetGroupId),
                  ),
                ),
                if (defaultTargetPlatform == TargetPlatform.android)
                  ListTile(
                    key: AddShortcutDialog.notificationsKey,
                    leading: const Icon(Icons.notifications_none),
                    title: const Text('Notifications'),
                    enabled: !_busy,
                    onTap: () => _run(
                      () => widget.state.addNotificationsShortcut(
                        widget.targetGroupId,
                      ),
                    ),
                  ),
                for (final provider in providers)
                  ListTile(
                    key: provider.id == 'command'
                        ? AddShortcutDialog.commandKey
                        : AddShortcutDialog.providerKey(provider.id),
                    leading: const Icon(Icons.link),
                    title: Text(provider.label),
                    enabled: !_busy,
                    onTap: provider.id == 'command'
                        ? _addCommand
                        : () => _run(
                            () => widget.state.beginShortcut(
                              provider.id,
                              widget.targetGroupId,
                            ),
                          ),
                  ),
              ],
            ),
          );
        },
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}

class _CommandShortcutDialog extends StatefulWidget {
  const _CommandShortcutDialog();

  @override
  State<_CommandShortcutDialog> createState() => _CommandShortcutDialogState();
}

class _CommandShortcutDialogState extends State<_CommandShortcutDialog> {
  final _label = TextEditingController();
  final _command = TextEditingController();

  @override
  void dispose() {
    _label.dispose();
    _command.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final ready =
        _label.text.trim().isNotEmpty && _command.text.trim().isNotEmpty;
    return AlertDialog(
      title: const Text('Command Shortcut'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            key: AddShortcutDialog.commandLabelKey,
            controller: _label,
            decoration: const InputDecoration(labelText: 'Name'),
            onChanged: (_) => setState(() {}),
          ),
          TextField(
            key: AddShortcutDialog.commandTextKey,
            controller: _command,
            decoration: const InputDecoration(
              labelText: 'Command',
              helperText: 'No shell is involved.',
            ),
            onChanged: (_) => setState(() {}),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: AddShortcutDialog.commandSaveKey,
          onPressed: ready
              ? () => Navigator.of(context).pop((
                  label: _label.text.trim(),
                  command: _command.text.trim(),
                ))
              : null,
          child: const Text('Add'),
        ),
      ],
    );
  }
}
