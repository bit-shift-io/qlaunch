import 'dart:async';

import 'package:flutter/painting.dart';
import 'package:flutter/scheduler.dart';

/// A stable identity includes the icon revision and physical display size.
class IconWarmRequest {
  const IconWarmRequest({
    required this.key,
    required this.estimatedBytes,
    required this.load,
  });
  final Object key;
  final int estimatedBytes;
  final Future<ImageProvider?> Function() load;
}

/// One speculative load/decode at a time, yielding to gestures and animations.
class IconWarmer {
  IconWarmer({
    void Function(Future<void> Function())? schedule,
    Future<void> Function(ImageProvider)? warm,
  }) : _schedule = schedule ?? _scheduleIdle,
       _warm = warm ?? warmIconImage;

  final void Function(Future<void> Function()) _schedule;
  final Future<void> Function(ImageProvider) _warm;
  List<IconWarmRequest> _queue = [];
  int _next = 0;
  int _generation = 0;
  bool _enabled = true;
  bool _busy = false;
  bool _disposed = false;
  bool Function()? _hasRoom;

  static void _scheduleIdle(Future<void> Function() task) {
    unawaited(SchedulerBinding.instance.scheduleTask(task, Priority.idle));
  }

  void replaceQueue(
    Iterable<IconWarmRequest> requests, {
    int maximumBytes = 32 << 20,
    int maximumEntries = 1000,
    bool Function()? hasRoom,
  }) {
    cancel();
    _hasRoom = hasRoom;
    final seen = <Object>{};
    var bytes = 0;
    _queue = [];
    for (final request in requests) {
      if (!seen.add(request.key)) continue;
      if (_queue.length >= maximumEntries ||
          bytes + request.estimatedBytes > maximumBytes) {
        break;
      }
      bytes += request.estimatedBytes;
      _queue.add(request);
    }
    _kick();
  }

  void setEnabled(bool enabled) {
    _enabled = enabled;
    _kick();
  }

  void cancel() {
    _generation++;
    _queue = [];
    _next = 0;
    _hasRoom = null;
  }

  void _kick() {
    if (_disposed || !_enabled || _busy || _next >= _queue.length) return;
    if (!(_hasRoom?.call() ?? true)) {
      cancel();
      return;
    }
    _busy = true;
    _schedule(() async {
      final generation = _generation;
      try {
        if (_disposed || !_enabled || _next >= _queue.length) return;
        if (!(_hasRoom?.call() ?? true)) {
          cancel();
          return;
        }
        final provider = await _queue[_next++].load();
        if (_disposed || !_enabled || generation != _generation) return;
        if (provider != null) {
          if (!(_hasRoom?.call() ?? true)) {
            cancel();
            return;
          }
          await _warm(provider);
        }
      } catch (_) {
        // Missing/bad images must not abort the rest of the pass. Cells have
        // their own placeholder and error handling.
      } finally {
        _busy = false;
        _kick();
      }
    });
  }

  void dispose() {
    _disposed = true;
    cancel();
  }
}

/// Resolve the provider, not just its bytes, and release the live listener.
Future<void> warmIconImage(ImageProvider provider) {
  final stream = provider.resolve(ImageConfiguration.empty);
  final done = Completer<void>();
  late final ImageStreamListener listener;
  listener = ImageStreamListener(
    (image, synchronousCall) {
      image.dispose();
      if (!done.isCompleted) done.complete();
    },
    onError: (Object error, StackTrace? stack) {
      if (!done.isCompleted) done.completeError(error, stack);
    },
  );
  stream.addListener(listener);
  return done.future.whenComplete(() => stream.removeListener(listener));
}
