import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';

import 'raster_image_store.dart';

typedef RasterScheduledTask = void Function();
typedef RasterSchedulerSeam = void Function(RasterScheduledTask task);

/// Runs one capture and completes when publication or failure is settled.
typedef RasterCapture = Future<void> Function();

class _Job {
  _Job({
    required this.key,
    required this.isCurrent,
    required this.capture,
    required this.queuedAt,
  });
  final RasterCacheKey key;
  final bool Function() isCurrent;
  final RasterCapture capture;
  final int queuedAt;
  final owners = <Object>{};
  bool started = false;
}

/// Deduplicated capture work. Visible jobs run post-paint, bounded per frame and
/// by a synchronous submission budget; speculative jobs run only when idle and
/// nothing visible is pending.
class RasterCaptureScheduler {
  RasterCaptureScheduler({
    this.maximumVisiblePerFrame = 4,
    this.submissionBudget = const Duration(milliseconds: 2),
    @visibleForTesting RasterSchedulerSeam? frameScheduler,
    @visibleForTesting RasterSchedulerSeam? idleScheduler,
    @visibleForTesting int Function()? clock,
  }) : _scheduleFrame = frameScheduler ?? _defaultFrame,
       _scheduleIdle = idleScheduler ?? _defaultIdle,
       _clock = clock ?? _defaultClock;

  final int maximumVisiblePerFrame;
  final Duration submissionBudget;
  final RasterSchedulerSeam _scheduleFrame;
  final RasterSchedulerSeam _scheduleIdle;
  final int Function() _clock;

  static int _defaultClock() => DateTime.now().microsecondsSinceEpoch;

  final _visible = <RasterCacheKey, _Job>{};
  final _speculative = <RasterCacheKey, _Job>{};

  bool _frameScheduled = false, _idleScheduled = false;
  bool _disposed = false,
      _speculationEnabled = true,
      _speculativeInFlight = false;
  int _submissions = 0, _cancellations = 0, _failures = 0, _waitMicros = 0;

  int get visibleQueued => _visible.values.where((j) => !j.started).length;
  int get speculativeQueued => _speculative.length;
  int get submissions => _submissions;
  int get cancellations => _cancellations;
  int get failures => _failures;
  int get waitMicros => _waitMicros;
  bool get speculationEnabled => _speculationEnabled;

  Map<String, Object> statistics() => {
    'visibleQueued': visibleQueued,
    'speculativeQueued': speculativeQueued,
    'inFlightVisible': _visible.values.where((j) => j.started).length,
    'submissions': _submissions,
    'cancellations': _cancellations,
    'failures': _failures,
    'waitMs': _waitMicros / 1000,
  };
  void resetStatistics() {
    _submissions = _cancellations = _failures = _waitMicros = 0;
  }

  void enqueueVisible({
    required Object owner,
    required RasterCacheKey key,
    required bool Function() isCurrent,
    required RasterCapture capture,
  }) {
    if (_disposed) return;
    _detachOwner(owner, keep: key);
    final existing = _visible[key];
    if (existing != null) {
      existing.owners.add(owner);
      return;
    }
    _visible[key] = _Job(
      key: key,
      isCurrent: isCurrent,
      capture: capture,
      queuedAt: _clock(),
    )..owners.add(owner);
    _scheduleFrameTask();
  }

  void enqueueSpeculative({
    required RasterCacheKey key,
    required bool Function() isCurrent,
    required RasterCapture capture,
  }) {
    if (_disposed || !_speculationEnabled || _speculative.containsKey(key)) {
      return;
    }
    _speculative[key] = _Job(
      key: key,
      isCurrent: isCurrent,
      capture: capture,
      queuedAt: _clock(),
    );
    _scheduleIdleTaskIfNeeded();
  }

  /// Removes [owner] from every visible job, dropping empty ones. A live second
  /// subscriber keeps the shared job alive.
  void cancelOwner(Object owner) => _detachOwner(owner);

  void cancelSpeculative() {
    _cancellations += _speculative.length;
    _speculative.clear();
  }

  void setSpeculationEnabled(bool enabled) {
    _speculationEnabled = enabled;
    if (enabled) _scheduleIdleTaskIfNeeded();
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _cancellations += _visible.length + _speculative.length;
    _visible.clear();
    _speculative.clear();
  }

  void _detachOwner(Object owner, {RasterCacheKey? keep}) {
    for (final job in _visible.values.toList()) {
      if (job.key == keep) continue;
      if (!job.owners.remove(owner)) continue;
      if (job.owners.isNotEmpty) continue;
      _visible.remove(job.key);
      if (!job.started) _cancellations++;
    }
  }

  void _scheduleFrameTask() {
    if (_frameScheduled || _disposed) return;
    _frameScheduled = true;
    _scheduleFrame(_runVisible);
  }

  void _runVisible() {
    _frameScheduled = false;
    if (_disposed) return;
    final budgetMicros = submissionBudget.inMicroseconds;
    var started = 0, syncMicros = 0;
    for (final job in _visible.values.toList()) {
      if (started >= maximumVisiblePerFrame) break;
      if (started > 0 && syncMicros >= budgetMicros) break;
      if (!_visible.containsKey(job.key) || job.started) continue;
      if (!job.isCurrent()) {
        _visible.remove(job.key);
        _cancellations++;
        continue;
      }
      job.started = true;
      _submissions++;
      _waitMicros += _clock() - job.queuedAt;
      final watch = Stopwatch()..start();
      late Future<void> future;
      try {
        future = job.capture();
      } catch (error) {
        future = Future<void>.error(error);
      }
      syncMicros += watch.elapsedMicroseconds;
      started++;
      _visible.remove(job.key);
      future
          .then<void>((_) {})
          .catchError((Object error) {
            _failures++;
          })
          .whenComplete(() {
            if (!_disposed && _visible.isEmpty) _scheduleIdleTaskIfNeeded();
          });
    }
    if (_visible.isNotEmpty) _scheduleFrameTask();
    if (_visible.isEmpty) _scheduleIdleTaskIfNeeded();
  }

  void _scheduleIdleTaskIfNeeded() {
    if (_idleScheduled || _disposed || !_speculationEnabled) return;
    if (_speculative.isEmpty || _visible.isNotEmpty || _speculativeInFlight) {
      return;
    }
    _idleScheduled = true;
    _scheduleIdle(_runSpeculative);
  }

  void _runSpeculative() {
    _idleScheduled = false;
    if (_disposed || !_speculationEnabled || _speculativeInFlight) return;
    if (_visible.isNotEmpty) {
      _scheduleFrameTask();
      return;
    }
    final job = _speculative.values.firstOrNull;
    if (job == null) return;
    _speculative.remove(job.key);
    if (!job.isCurrent()) {
      _scheduleIdleTaskIfNeeded();
      return;
    }
    _speculativeInFlight = true;
    _submissions++;
    _waitMicros += _clock() - job.queuedAt;
    late Future<void> future;
    try {
      future = job.capture();
    } catch (error) {
      future = Future<void>.error(error);
    }
    future
        .then<void>((_) {})
        .catchError((Object error) {
          _failures++;
        })
        .whenComplete(() {
          _speculativeInFlight = false;
          if (!_disposed && _speculationEnabled) _scheduleIdleTaskIfNeeded();
        });
  }

  static void _defaultFrame(RasterScheduledTask task) {
    final binding = SchedulerBinding.instance;
    binding.addPostFrameCallback((_) => task());
    binding.ensureVisualUpdate();
  }

  static void _defaultIdle(RasterScheduledTask task) {
    SchedulerBinding.instance.scheduleTask(task, Priority.idle);
  }
}
