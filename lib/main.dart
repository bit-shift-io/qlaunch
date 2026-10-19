import 'dart:async';

import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/frb_generated.dart';
import 'package:qlaunch/src/state/launcher_backend.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/home_screen.dart';
import 'package:qlaunch/src/ui/settings_screen.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(const QLaunchApp());
}

/// The application shell: it owns the core session and the one launcher state
/// every screen reads.
class QLaunchApp extends StatefulWidget {
  const QLaunchApp({super.key, this.backendOverride});

  /// The core to talk to. `null` uses the real one; a test supplies a fake.
  final LauncherBackend? backendOverride;

  @override
  State<QLaunchApp> createState() => _QLaunchAppState();
}

class _QLaunchAppState extends State<QLaunchApp> {
  late final LauncherState _state;

  @override
  void initState() {
    super.initState();
    _state = LauncherState(widget.backendOverride ?? const RustBackend());
    unawaited(_state.load());
  }

  @override
  void dispose() {
    _state.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'QLaunch',
      // The launcher is a dark surface, so the whole app defaults to a dark
      // theme; the settings screen and its dialogs follow it.
      theme: ThemeData.dark(useMaterial3: true),
      themeMode: ThemeMode.dark,
      // §8.1's routes are declared here: the settings screen is pushed by name
      // so the left drawer (Plan 3) can reach it without holding the state.
      // Nothing pushes it yet — the drawer is deferred (F3).
      onGenerateRoute: (settings) {
        if (settings.name != SettingsScreen.routeName) return null;
        if (_state.snapshot == null) return null;
        return MaterialPageRoute<void>(
          settings: settings,
          builder: (context) => SettingsScreen(state: _state),
        );
      },
      home: ListenableBuilder(
        listenable: _state,
        builder: (context, _) {
          if (_state.snapshot != null) return HomeScreen(state: _state);
          final error = _state.error;
          if (error == null) {
            return const Scaffold(
              body: Center(child: CircularProgressIndicator()),
            );
          }
          // A state with no snapshot and an error is a failed start, not an
          // empty launcher: there is nothing to show the user but the reason.
          return Scaffold(
            body: Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Text('QLaunch could not read its applications.'),
                    const SizedBox(height: 12),
                    Text(error, textAlign: TextAlign.center),
                    const SizedBox(height: 12),
                    FilledButton(
                      onPressed: () => unawaited(_state.load()),
                      child: const Text('Try again'),
                    ),
                  ],
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}
