import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:integration_test/integration_test_driver.dart';

/// Sends real native Back presses only when the device test requests one.
/// ANDROID_SERIAL should match flutter drive's -d argument.
Future<void> main() async {
  final serial = Platform.environment['ANDROID_SERIAL'];
  final device = <String>[
    if (serial != null) ...['-s', serial],
  ];
  final logcat = await Process.start('adb', [
    ...device,
    'logcat',
    '-v',
    'brief',
    '-T',
    '1',
    'flutter:I',
    '*:S',
  ]);
  final handled = <String>{};
  final subscription = logcat.stdout
      .transform(utf8.decoder)
      .transform(const LineSplitter())
      .listen((line) {
        final id = RegExp(r'PAGE_CONTROL_BACK (\d+)')
            .firstMatch(line)
            ?.group(1);
        if (id == null || !handled.add(id)) return;
        unawaited(
          Process.run('adb', [
            ...device,
            'shell',
            'input',
            'keyevent',
            'KEYCODE_BACK',
          ]).then((result) {
            if (result.exitCode != 0)
              stderr.writeln('Native Back failed: ${result.stderr}');
          }),
        );
      });
  await integrationDriver(
    writeResponseOnFailure: true,
    responseDataCallback: (data) async {
      await subscription.cancel();
      logcat.kill();
      await writeResponseData(data);
    },
  );
}
