import 'package:flutter/material.dart';
import 'package:qlaunch/src/rust/api/api.dart' as rust;
import 'package:qlaunch/src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(const QLaunchApp());
}

class QLaunchApp extends StatelessWidget {
  const QLaunchApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'qlaunch',
      home: Scaffold(
        body: Center(
          child: FutureBuilder<String>(
            future: rust.version(),
            builder: (context, snapshot) {
              if (snapshot.hasError) {
                return Text('${snapshot.error}');
              }
              return Text(snapshot.data ?? '');
            },
          ),
        ),
      ),
    );
  }
}
