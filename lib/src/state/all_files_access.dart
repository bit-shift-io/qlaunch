import 'package:flutter/services.dart';

/// Android "All files access" (`MANAGE_EXTERNAL_STORAGE`), needed to list
/// backup files in Downloads that this install does not own. A missing plugin
/// (tests, desktop) is treated as granted so non-Android runs are unaffected.
class AllFilesAccess {
  static const channel = MethodChannel('bitshift.qlaunch/storage');

  static Future<bool> isGranted() async {
    try {
      return await channel.invokeMethod<bool>('hasAllFilesAccess') ?? false;
    } on MissingPluginException {
      return true;
    } on PlatformException {
      return false;
    }
  }

  static Future<void> request() async {
    try {
      await channel.invokeMethod<void>('requestAllFilesAccess');
    } on MissingPluginException {
      // Nothing to open outside Android.
    } on PlatformException {
      // The settings screen could not be opened; isGranted() stays false and
      // the caller explains what to do.
    }
  }
}
