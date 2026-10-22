import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/state/icon_warmer.dart';
import 'package:qlaunch/src/ui/raw_rgba_image.dart';

void main() {
  test('warmer serialises, deduplicates, pauses and reprioritises', () async {
    final scheduled = <Future<void> Function()>[];
    final seen = <String>[];
    final gate = Completer<void>();
    final warmer = IconWarmer(
      schedule: scheduled.add,
      warm: (_) async {
        await gate.future;
      },
    );
    IconWarmRequest request(String id) => IconWarmRequest(
      key: id,
      estimatedBytes: 4,
      load: () async {
        seen.add(id);
        return const AssetImage('unused');
      },
    );
    warmer.replaceQueue([request('a'), request('a'), request('b')]);
    final first = scheduled.removeAt(0)();
    await Future<void>.delayed(Duration.zero);
    expect(seen, ['a']);
    expect(scheduled, isEmpty);
    warmer.setEnabled(false);
    gate.complete();
    await first;
    expect(scheduled, isEmpty);
    warmer.setEnabled(true);
    await scheduled.removeAt(0)();
    expect(seen, ['a', 'b']);
    expect(scheduled, isEmpty);
    warmer.dispose();
  });

  test('budget ends a pass and cancellation ignores scheduled work', () async {
    final scheduled = <Future<void> Function()>[];
    var loads = 0;
    final warmer = IconWarmer(schedule: scheduled.add, warm: (_) async {});
    final requests = List.generate(
      10,
      (i) => IconWarmRequest(
        key: i,
        estimatedBytes: 4,
        load: () async {
          loads++;
          return null;
        },
      ),
    );
    warmer.replaceQueue(requests, maximumBytes: 8, maximumEntries: 2);
    while (scheduled.isNotEmpty) {
      await scheduled.removeAt(0)();
    }
    expect(loads, 2);
    warmer.replaceQueue(requests);
    warmer.cancel();
    while (scheduled.isNotEmpty) {
      await scheduled.removeAt(0)();
    }
    expect(loads, 2);
    warmer.dispose();
  });

  test('actual budget feedback ends the pass before churn', () async {
    final scheduled = <Future<void> Function()>[];
    var loads = 0;
    var hasRoom = true;
    final warmer = IconWarmer(schedule: scheduled.add, warm: (_) async {});
    final requests = List.generate(
      6,
      (i) => IconWarmRequest(
        key: i,
        estimatedBytes: 4,
        load: () async {
          loads++;
          // The real caches only report fullness once entries are retained.
          if (loads == 2) hasRoom = false;
          return null;
        },
      ),
    );
    warmer.replaceQueue(requests, hasRoom: () => hasRoom);
    while (scheduled.isNotEmpty) {
      await scheduled.removeAt(0)();
    }
    expect(loads, 2);
    warmer.dispose();
  });

  test(
    'errors and obsolete in-flight results do not continue disposed queue',
    () async {
      final scheduled = <Future<void> Function()>[];
      final gate = Completer<ImageProvider?>();
      var warmed = 0;
      final warmer = IconWarmer(
        schedule: scheduled.add,
        warm: (_) async {
          warmed++;
        },
      );
      warmer.replaceQueue([
        IconWarmRequest(
          key: 1,
          estimatedBytes: 4,
          load: () async => throw StateError('bad'),
        ),
        IconWarmRequest(key: 2, estimatedBytes: 4, load: () => gate.future),
        IconWarmRequest(key: 3, estimatedBytes: 4, load: () async => null),
      ]);
      await scheduled.removeAt(0)();
      final pending = scheduled.removeAt(0)();
      warmer.dispose();
      gate.complete(const AssetImage('unused'));
      await pending;
      expect(warmed, 0);
      expect(scheduled, isEmpty);
    },
  );

  testWidgets(
    'warming resolves RGBA into decoded cache before any Image mounts',
    (tester) async {
      final provider = RawRgbaImage(Uint8List.fromList([0, 0, 0, 255]), 1, 1);
      await tester.runAsync(() => warmIconImage(provider));
      final key = await provider.obtainKey(ImageConfiguration.empty);
      expect(PaintingBinding.instance.imageCache.containsKey(key), isTrue);
      expect(
        PaintingBinding.instance.imageCache.statusForKey(key).keepAlive,
        isTrue,
      );
      provider.evict();
    },
  );
}
