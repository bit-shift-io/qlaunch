import 'dart:convert';
import 'dart:typed_data';
import 'dart:async';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model/icon.dart';
import 'package:qlaunch/src/state/icon_cache.dart';
import 'package:qlaunch/src/state/launcher_state.dart';

import 'fake_backend.dart';

// A 1x1 PNG.
final Uint8List _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
);

void main() {
  test(
    'state cached misses complete rather than parking on catchError',
    () async {
      final backend = FakeBackend();
      final state = LauncherState(backend);
      expect(await state.iconFor('missing', '', 48), isNull);
      expect(
        await state
            .iconFor('missing', '', 48)
            .timeout(const Duration(milliseconds: 50)),
        isNull,
      );
      state.dispose();
      await backend.close();
    },
  );
  late FakeBackend fake;
  late IconCache cache;

  setUp(() {
    fake = FakeBackend();
    cache = IconCache(fake);
  });

  tearDown(() => fake.close());

  test(
    'a second resolve for the same key does not call the backend again',
    () async {
      fake.icons[(package: 'p', activity: 'a')] = IconImage(
        format: IconFormat.png,
        width: 0,
        height: 0,
        bytes: _png,
      );
      final first = await cache.resolve('p', 'a', 48);
      final second = await cache.resolve('p', 'a', 48);
      expect(identical(first, second), isTrue);
      expect(fake.iconCalls, 1);
    },
  );

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

  test('source budget evicts least recently used providers', () async {
    cache = IconCache(fake, maximumBytes: 8, maximumEntries: 3);
    for (final package in ['a', 'b', 'c']) {
      fake.icons[(package: package, activity: '')] = IconImage(
        format: IconFormat.rgba,
        width: 1,
        height: 1,
        bytes: Uint8List.fromList([0, 0, 0, 255]),
      );
    }
    final a = await cache.resolve('a', '', 1);
    await cache.resolve('b', '', 1);
    expect(cache.peek('a', '', 1), same(a));
    await cache.resolve('c', '', 1);
    expect(cache.retainedBytes, 8);
    expect(cache.peek('b', '', 1), isNull);
    expect(cache.peek('a', '', 1), same(a));
  });

  test('null entries are bounded and failures retry', () async {
    cache = IconCache(fake, maximumEntries: 2);
    for (final p in ['a', 'b', 'c']) {
      await cache.resolve(p, '', 1);
    }
    expect(cache.entryCount, 2);
    await cache.resolve('a', '', 1);
    expect(fake.iconCalls, 4);
    fake.failure = StateError('failed');
    await expectLater(cache.resolve('error', '', 1), throwsStateError);
    fake.failure = null;
    expect(await cache.resolve('error', '', 1), isNull);
  });

  test('invalidation prevents pending loads resurrecting entries', () async {
    final backend = _HeldIconBackend();
    final heldCache = IconCache(backend);
    final first = heldCache.resolve('a', '', 1);
    expect(heldCache.resolve('a', '', 1), same(first));
    heldCache.invalidateApp('a', '');
    final second = heldCache.resolve('a', '', 1);
    backend.responses[0].complete(
      IconImage(format: IconFormat.png, width: 0, height: 0, bytes: _png),
    );
    await first;
    expect(heldCache.entryCount, 0);
    backend.responses[1].complete(null);
    await second;
    expect(heldCache.entryCount, 1);
    await backend.close();
  });

  test('shortcuts reuse resized providers and invalidate explicitly', () async {
    fake.itemIcons['shortcut'] = _png;
    final first = await cache.resolveItem('shortcut', 'icon.png', 48);
    expect(first, isA<ResizeImage>());
    expect((first as ResizeImage).width, 48);
    expect(await cache.resolveItem('shortcut', 'icon.png', 48), same(first));
    expect(fake.calls.where((c) => c == 'itemIcon(shortcut)').length, 1);
    cache.invalidateItem('shortcut');
    expect(
      await cache.resolveItem('shortcut', 'icon.png', 48),
      isNot(same(first)),
    );
    cache.clear();
    expect(cache.retainedBytes, 0);
    expect(cache.entryCount, 0);
  });
}

class _HeldIconBackend extends FakeBackend {
  final responses = <Completer<IconImage?>>[];
  @override
  Future<IconImage?> iconFor(String package, String activity, int sizePx) {
    final response = Completer<IconImage?>();
    responses.add(response);
    return response.future;
  }
}
