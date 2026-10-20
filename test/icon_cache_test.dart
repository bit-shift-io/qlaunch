import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model/icon.dart';
import 'package:qlaunch/src/state/icon_cache.dart';

import 'fake_backend.dart';

// A 1x1 PNG.
final Uint8List _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
);

void main() {
  late FakeBackend fake;
  late IconCache cache;

  setUp(() {
    fake = FakeBackend();
    cache = IconCache(fake);
  });

  tearDown(() => fake.close());

  test('a second resolve for the same key does not call the backend again', () async {
    fake.icons[(package: 'p', activity: 'a')] =
        IconImage(format: IconFormat.png, width: 0, height: 0, bytes: _png);
    final first = await cache.resolve('p', 'a', 48);
    final second = await cache.resolve('p', 'a', 48);
    expect(identical(first, second), isTrue);
    expect(fake.iconCalls, 1);
  });

  test('a different size is a different key', () async {
    await cache.resolve('p', 'a', 48);
    await cache.resolve('p', 'a', 96);
    expect(fake.iconCalls, 2);
  });

  test('a missing icon resolves null and is cached as a miss', () async {
    expect(await cache.resolve('p', 'a', 48), isNull);
    expect(await cache.resolve('p', 'a', 48), isNull);
    expect(fake.iconCalls, 1);
  });
}
