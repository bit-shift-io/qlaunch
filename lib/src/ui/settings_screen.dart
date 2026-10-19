import 'dart:async';

import 'package:flutter/material.dart' hide Orientation;
import 'package:qlaunch/src/rust/settings.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/group_editor_screen.dart';

/// Every setting in §8.6, grouped as Display / Window / Icon / Text / Advanced.
///
/// The screen holds no setting of its own: every value it draws comes from the
/// snapshot the core returned, so what the user sees after a change is what was
/// actually saved (a value the core clamped shows up clamped). The one
/// exception is the number a slider is being dragged to, which is kept only
/// until the patch comes back.
class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key, required this.state});

  /// Named so the left drawer (Plan 3) can push it without holding the state.
  static const routeName = '/settings';

  final LauncherState state;

  /// One per row of §8.6's settings table, so a control that is forgotten is a
  /// missing key in this screen's test rather than a silent omission.
  static Key controlKey(SettingField field) =>
      ValueKey('setting.${field.name}.control');

  /// One per choice of a choice control, so a test can tap an exact value.
  static Key choiceKey(SettingField field, Object value) =>
      ValueKey('setting.${field.name}.$value');

  static const restartKey = ValueKey('setting.restart');
  static const factoryResetKey = ValueKey('setting.factoryReset');
  static const groupsKey = ValueKey('setting.groups');

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

/// The settings §8.6 lists, one per control. The two actions at the end of that
/// table are not settings and are not here.
enum SettingField {
  gridColumnsPortrait,
  gridColumnsLandscape,
  layout,
  tabVisibility,
  wallpaperAlpha,
  statusBarVisible,
  iconsVisible,
  iconSize,
  iconAlpha,
  labelsVisible,
  labelSize,
  labelTypeface,
  labelColour,
  labelShadow,
  orientation,
}

class _SettingsScreenState extends State<SettingsScreen> {
  /// Numbers a slider is currently being dragged to, by field.
  final Map<SettingField, double> _dragging = {};

  void _patch(SettingsPatch patch, {SettingField? dragging}) {
    unawaited(
      widget.state.updateSettings(patch).whenComplete(() {
        if (!mounted || dragging == null) return;
        // The snapshot has arrived: stop showing the dragged number and let
        // the saved one take over.
        setState(() => _dragging.remove(dragging));
      }),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Launcher Settings')),
      body: ListenableBuilder(
        listenable: widget.state,
        builder: (context, _) {
          final settings = widget.state.snapshot?.settings;
          if (settings == null) {
            return const Center(child: CircularProgressIndicator());
          }
          return SingleChildScrollView(
            // Every control is built, whether or not it is on screen: a screen
            // as short as this does not need a lazy list, and this screen's
            // test asserts one control per setting, which it could not do for
            // children that had never been built.
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _header('Display'),
                _number(
                  SettingField.gridColumnsPortrait,
                  'Portrait columns',
                  settings.gridColumnsPortrait.toDouble(),
                  1,
                  12,
                  (v) => SettingsPatch(gridColumnsPortrait: v.round()),
                ),
                _number(
                  SettingField.gridColumnsLandscape,
                  'Landscape columns',
                  settings.gridColumnsLandscape.toDouble(),
                  1,
                  12,
                  (v) => SettingsPatch(gridColumnsLandscape: v.round()),
                ),
                _choice(
                  SettingField.layout,
                  'Cell layout',
                  LayoutStyle.values,
                  settings.layout,
                  _layoutName,
                  (v) => SettingsPatch(layout: v),
                ),
                _choice(
                  SettingField.tabVisibility,
                  'Tab strip',
                  TabVisibility.values,
                  settings.tabVisibility,
                  _tabVisibilityName,
                  (v) => SettingsPatch(tabVisibility: v),
                ),

                _header('Window'),
                _number(
                  SettingField.wallpaperAlpha,
                  'Wallpaper dimming',
                  settings.wallpaperAlpha.toDouble(),
                  0,
                  100,
                  (v) => SettingsPatch(wallpaperAlpha: v.round()),
                ),
                _switch(
                  SettingField.statusBarVisible,
                  'Show the status bar',
                  settings.statusBarVisible,
                  (v) => SettingsPatch(statusBarVisible: v),
                ),
                _choice(
                  SettingField.orientation,
                  'Requested orientation',
                  Orientation.values,
                  settings.orientation,
                  _orientationName,
                  (v) => SettingsPatch(orientation: v),
                ),

                _header('Icon'),
                _switch(
                  SettingField.iconsVisible,
                  'Show icons',
                  settings.iconsVisible,
                  (v) => SettingsPatch(iconsVisible: v),
                ),
                _number(
                  SettingField.iconSize,
                  'Icon size',
                  settings.iconSize.toDouble(),
                  0,
                  100,
                  (v) => SettingsPatch(iconSize: v.round()),
                ),
                _number(
                  SettingField.iconAlpha,
                  'Icon opacity',
                  settings.iconAlpha.toDouble(),
                  0,
                  100,
                  (v) => SettingsPatch(iconAlpha: v.round()),
                ),

                _header('Text'),
                _switch(
                  SettingField.labelsVisible,
                  'Show labels',
                  settings.labelsVisible,
                  (v) => SettingsPatch(labelsVisible: v),
                ),
                _number(
                  SettingField.labelSize,
                  'Label size',
                  settings.labelSize.toDouble(),
                  0,
                  100,
                  (v) => SettingsPatch(labelSize: v.round()),
                ),
                _choice(
                  SettingField.labelTypeface,
                  'Typeface',
                  Typeface.values,
                  settings.labelTypeface,
                  _typefaceName,
                  (v) => SettingsPatch(labelTypeface: v),
                ),
                _choice(
                  SettingField.labelColour,
                  'Colour',
                  LabelColour.values,
                  settings.labelColour,
                  _labelColourName,
                  (v) => SettingsPatch(labelColour: v),
                ),
                _switch(
                  SettingField.labelShadow,
                  'Label shadow',
                  settings.labelShadow,
                  (v) => SettingsPatch(labelShadow: v),
                ),

                _header('Groups'),
                ListTile(
                  key: SettingsScreen.groupsKey,
                  title: const Text('Groups'),
                  subtitle: const Text(
                    'Add, rename, reorder and fill the tabs.',
                  ),
                  onTap: () => unawaited(
                    Navigator.of(context).push(
                      MaterialPageRoute<void>(
                        builder: (context) =>
                            GroupEditorScreen(state: widget.state),
                      ),
                    ),
                  ),
                ),

                _header('Advanced'),
                ListTile(
                  key: SettingsScreen.restartKey,
                  title: const Text('Restart Launcher'),
                  subtitle: const Text(
                    'Re-reads the saved layout and the installed applications.',
                  ),
                  onTap: () => unawaited(widget.state.load()),
                ),
                ListTile(
                  key: SettingsScreen.factoryResetKey,
                  title: const Text('Factory Reset'),
                  subtitle: const Text(
                    'Wipes every group, item and setting, after confirmation.',
                  ),
                  onTap: _factoryReset,
                ),
                const SizedBox(height: 32),
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _header(String text) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 24, 16, 0),
    child: Text(
      text,
      style: Theme.of(context).textTheme.titleSmall
          ?.copyWith(fontWeight: FontWeight.w600),
    ),
  );

  Widget _label(String text, String value) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
    child: Row(
      children: [
        Expanded(child: Text(text)),
        Text(value, style: Theme.of(context).textTheme.bodySmall),
      ],
    ),
  );

  Widget _number(
    SettingField field,
    String label,
    double value,
    double min,
    double max,
    SettingsPatch Function(double) patch,
  ) {
    final shown = (_dragging[field] ?? value).clamp(min, max);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _label(label, '${shown.round()}'),
        Slider(
          key: SettingsScreen.controlKey(field),
          value: shown,
          min: min,
          max: max,
          divisions: (max - min).round(),
          label: '${shown.round()}',
          onChanged: (v) => setState(() => _dragging[field] = v),
          // One request per gesture rather than one per pixel.
          onChangeEnd: (v) => _patch(patch(v), dragging: field),
        ),
      ],
    );
  }

  Widget _switch(
    SettingField field,
    String label,
    bool value,
    SettingsPatch Function(bool) patch,
  ) => SwitchListTile(
    key: SettingsScreen.controlKey(field),
    title: Text(label),
    value: value,
    onChanged: (v) => _patch(patch(v)),
  );

  Widget _choice<T extends Object>(
    SettingField field,
    String label,
    List<T> values,
    T selected,
    String Function(T) name,
    SettingsPatch Function(T) patch,
  ) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      _label(label, ''),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Wrap(
          key: SettingsScreen.controlKey(field),
          spacing: 8,
          children: [
            for (final value in values)
              ChoiceChip(
                key: SettingsScreen.choiceKey(field, value),
                label: Text(name(value)),
                selected: value == selected,
                onSelected: (_) => _patch(patch(value)),
              ),
          ],
        ),
      ),
    ],
  );

  String _layoutName(LayoutStyle value) => switch (value) {
    LayoutStyle.tiles => 'Tiles',
    LayoutStyle.list => 'List',
    LayoutStyle.centred => 'Centred',
  };

  String _tabVisibilityName(TabVisibility value) => switch (value) {
    TabVisibility.always => 'Always',
    TabVisibility.landscapeOnly => 'Landscape',
    TabVisibility.portraitOnly => 'Portrait',
    TabVisibility.never => 'Never',
  };

  String _typefaceName(Typeface value) => switch (value) {
    Typeface.normal => 'Normal',
    Typeface.sans => 'Sans',
    Typeface.serif => 'Serif',
    Typeface.monospace => 'Mono',
  };

  String _labelColourName(LabelColour value) => switch (value) {
    LabelColour.white => 'White',
    LabelColour.black => 'Black',
    LabelColour.grey => 'Grey',
  };

  String _orientationName(Orientation value) => switch (value) {
    Orientation.auto => 'Auto',
    Orientation.landscape => 'Landscape',
    Orientation.portrait => 'Portrait',
  };

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
    if (confirmed != true || !mounted) return;
    // There is nothing to call: the bridge has no `factory_reset`, and the
    // spec's own §8.9 surface does not list one.
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text(
          'Factory reset is not available yet: the core command for it is '
          'still to be added. Nothing was changed.',
        ),
      ),
    );
  }
}
