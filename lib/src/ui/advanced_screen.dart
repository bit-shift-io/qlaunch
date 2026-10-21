import 'dart:async';

import 'package:flutter/material.dart';
import 'package:qlaunch/src/state/launcher_state.dart';

/// The less-used, mostly destructive actions, kept a screen away from the
/// settings so a stray tap cannot restart or wipe the launcher.
class AdvancedScreen extends StatefulWidget {
  const AdvancedScreen({super.key, required this.state});

  static const routeName = '/settings/advanced';

  static const restartKey = ValueKey('advanced.restart');
  static const backupKey = ValueKey('advanced.backup');
  static const restoreKey = ValueKey('advanced.restore');
  static const factoryResetKey = ValueKey('advanced.factoryReset');

  final LauncherState state;

  @override
  State<AdvancedScreen> createState() => _AdvancedScreenState();
}

class _AdvancedScreenState extends State<AdvancedScreen> {
  Future<void> _backup() async {
    final name = await widget.state.backupConfig();
    if (!mounted) return;
    _say(
      name == null
          ? 'Backup failed: ${widget.state.error}'
          : 'Backup saved to Downloads/$name.',
    );
  }

  Future<void> _restore() async {
    final backups = await widget.state.listBackups();
    if (!mounted) return;
    if (backups.isEmpty) {
      _say('No backups found in Downloads.');
      return;
    }
    final chosen = await showDialog<String>(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('Restore config'),
        children: [
          for (final name in backups)
            SimpleDialogOption(
              onPressed: () => Navigator.of(context).pop(name),
              child: Text(name),
            ),
        ],
      ),
    );
    if (chosen == null || !mounted) return;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Restore config'),
        content: Text(
          'This replaces your current layout and settings with $chosen.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Restore'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    await widget.state.restoreConfig(chosen);
    if (!mounted) return;
    _say(
      widget.state.error == null
          ? 'Config restored from $chosen.'
          : 'Could not restore: ${widget.state.error}',
    );
  }

  Future<void> _factoryReset() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Factory Reset'),
        content: const Text(
          'This wipes every group, item and setting. It cannot be undone.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Reset'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    await widget.state.factoryReset();
    if (!mounted) return;
    if (widget.state.error == null) {
      _say('The launcher was reset.');
    }
  }

  void _say(String message) {
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Advanced')),
      body: ListView(
        children: [
          ListTile(
            key: AdvancedScreen.restartKey,
            title: const Text('Restart Launcher'),
            subtitle: const Text(
              'Re-reads the saved layout and the installed applications.',
            ),
            onTap: () => unawaited(widget.state.load()),
          ),
          ListTile(
            key: AdvancedScreen.backupKey,
            title: const Text('Backup config'),
            subtitle: const Text(
              'Saves the whole layout to a dated file in Downloads.',
            ),
            onTap: _backup,
          ),
          ListTile(
            key: AdvancedScreen.restoreKey,
            title: const Text('Restore config'),
            subtitle: const Text(
              'Replaces the layout with a backup from Downloads.',
            ),
            onTap: _restore,
          ),
          ListTile(
            key: AdvancedScreen.factoryResetKey,
            title: const Text('Factory Reset'),
            subtitle: const Text(
              'Wipes every group, item and setting, after confirmation.',
            ),
            onTap: _factoryReset,
          ),
        ],
      ),
    );
  }
}
