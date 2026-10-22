import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/raster_capture_scheduler.dart';
import 'package:qlaunch/src/ui/raster_image_store.dart';

RasterCacheKey key(int id) =>
    RasterCacheKey(generation: 0, kind: 'tile', owner: 'o$id', pixels: id);

void main() {
  late List<RasterScheduledTask> frames, idles;
  late int now;
  late RasterCaptureScheduler scheduler;
  final captured = <RasterCacheKey>[];
  final completer = <Future<void> Function()>[];

  RasterCaptureScheduler build({
    int maximumVisiblePerFrame = 4,
    Duration budget = const Duration(milliseconds: 2),
  }) => RasterCaptureScheduler(
    maximumVisiblePerFrame: maximumVisiblePerFrame,
    submissionBudget: budget,
    frameScheduler: (task) => frames.add(task),
    idleScheduler: (task) => idles.add(task),
    clock: () => now,
  );

  void runFrame() {
    for (final task in frames.toList()) {
      frames.remove(task);
      task();
    }
  }

  void runIdle() {
    for (final task in idles.toList()) {
      idles.remove(task);
      task();
    }
  }

  Future<void> settle() => Future<void>.delayed(Duration.zero);

  setUp(() {
    frames = [];
    idles = [];
    now = 1000;
    captured.clear();
    completer.clear();
    scheduler = build();
  });
  tearDown(() => scheduler.dispose());

  test('deduplicates by key while a second subscriber keeps it alive', () {
    scheduler.enqueueVisible(
      owner: 'a',
      key: key(1),
      isCurrent: () => true,
      capture: () async => captured.add(key(1)),
    );
    scheduler.enqueueVisible(
      owner: 'b',
      key: key(1),
      isCurrent: () => true,
      capture: () async => captured.add(key(1)),
    );
    scheduler.cancelOwner('a');
    expect(scheduler.visibleQueued, 1);
    runFrame();
    expect(captured, [key(1)]);
    expect(scheduler.submissions, 1);
  });

  test('a newer request for one owner replaces its obsolete work', () {
    scheduler.enqueueVisible(
      owner: 'a',
      key: key(1),
      isCurrent: () => true,
      capture: () async => captured.add(key(1)),
    );
    scheduler.enqueueVisible(
      owner: 'a',
      key: key(2),
      isCurrent: () => true,
      capture: () async => captured.add(key(2)),
    );
    runFrame();
    expect(captured, [key(2)]);
    expect(scheduler.cancellations, 1);
  });

  test('bounded submissions per frame, plus a synchronous budget', () {
    scheduler.dispose();
    scheduler = build(maximumVisiblePerFrame: 2);
    for (var i = 0; i < 5; i++) {
      scheduler.enqueueVisible(
        owner: 'o$i',
        key: key(i),
        isCurrent: () => true,
        capture: () async => captured.add(key(i)),
      );
    }
    runFrame();
    expect(scheduler.submissions, 2);
    runFrame();
    expect(scheduler.submissions, 4);
    runFrame();
    expect(scheduler.submissions, 5);
    expect(captured.length, 5);
  });

  test('the budget stops submissions within one frame', () async {
    scheduler.dispose();
    scheduler = build(maximumVisiblePerFrame: 10);
    for (var i = 0; i < 3; i++) {
      scheduler.enqueueVisible(
        owner: 'o$i',
        key: key(i),
        isCurrent: () => true,
        capture: () async {
          final until = DateTime.now().add(const Duration(milliseconds: 3));
          while (DateTime.now().isBefore(until)) {}
          captured.add(key(i));
        },
      );
    }
    runFrame();
    expect(scheduler.submissions, 1);
    await settle();
    runFrame();
    runFrame();
    expect(scheduler.submissions, 3);
  });

  test('visible work wins over speculative work', () async {
    scheduler.enqueueSpeculative(
      key: key(9),
      isCurrent: () => true,
      capture: () async => captured.add(key(9)),
    );
    scheduler.enqueueVisible(
      owner: 'a',
      key: key(1),
      isCurrent: () => true,
      capture: () async => captured.add(key(1)),
    );
    runIdle();
    expect(captured, isEmpty);
    runFrame();
    expect(captured, [key(1)]);
    await settle();
    runIdle();
    expect(captured, [key(1), key(9)]);
  });

  test('stale visible and speculative work is not captured', () {
    scheduler.enqueueVisible(
      owner: 'a',
      key: key(1),
      isCurrent: () => false,
      capture: () async => captured.add(key(1)),
    );
    scheduler.enqueueSpeculative(
      key: key(2),
      isCurrent: () => false,
      capture: () async => captured.add(key(2)),
    );
    runFrame();
    runIdle();
    expect(captured, isEmpty);
    expect(scheduler.cancellations, 1);
  });

  test('disabled speculation queues nothing and hold cancels all', () {
    scheduler.setSpeculationEnabled(false);
    scheduler.enqueueSpeculative(
      key: key(1),
      isCurrent: () => true,
      capture: () async => captured.add(key(1)),
    );
    expect(scheduler.speculativeQueued, 0);
    scheduler.setSpeculationEnabled(true);
    scheduler.enqueueSpeculative(
      key: key(1),
      isCurrent: () => true,
      capture: () async => captured.add(key(1)),
    );
    expect(scheduler.speculativeQueued, 1);
    scheduler.cancelSpeculative();
    expect(scheduler.speculativeQueued, 0);
    runIdle();
    expect(captured, isEmpty);
  });

  test('speculation keeps one in-flight job and waits for it', () async {
    for (var i = 0; i < 3; i++) {
      scheduler.enqueueSpeculative(
        key: key(i),
        isCurrent: () => true,
        capture: () async {
          captured.add(key(i));
          await Future<void>.delayed(const Duration(milliseconds: 5));
        },
      );
    }
    runIdle();
    expect(scheduler.submissions, 1);
    await Future<void>.delayed(const Duration(milliseconds: 20));
    runIdle();
    expect(scheduler.submissions, 2);
    await Future<void>.delayed(const Duration(milliseconds: 20));
    runIdle();
    expect(scheduler.submissions, 3);
  });

  test('a failure releases the slot so the queue makes progress', () async {
    scheduler.enqueueVisible(
      owner: 'a',
      key: key(1),
      isCurrent: () => true,
      capture: () async => throw StateError('boom'),
    );
    scheduler.enqueueVisible(
      owner: 'b',
      key: key(2),
      isCurrent: () => true,
      capture: () async => captured.add(key(2)),
    );
    runFrame();
    expect(captured, [key(2)]);
    await settle();
    expect(scheduler.failures, 1);
  });

  test('dispose stops all further captures', () {
    scheduler.enqueueVisible(
      owner: 'a',
      key: key(1),
      isCurrent: () => true,
      capture: () async => captured.add(key(1)),
    );
    scheduler.dispose();
    runFrame();
    expect(captured, isEmpty);
  });

  test('wait time is accounted from enqueue to submission', () {
    scheduler.enqueueVisible(
      owner: 'a',
      key: key(1),
      isCurrent: () => true,
      capture: () async => captured.add(key(1)),
    );
    now += 250;
    runFrame();
    expect(scheduler.waitMicros, 250);
  });
}
