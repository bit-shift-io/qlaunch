import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

/// An [ImageProvider] for an icon whose bytes are already raw RGBA pixels.
///
/// Android hands the core a bitmap it has just drawn, so sending those pixels
/// straight through avoids encoding a PNG on one side and decoding it on the
/// other. The framework's own [ImageCache] caches the decoded image against
/// this provider, so the pixels are decoded at most once per key.
class RawRgbaImage extends ImageProvider<RawRgbaImage> {
  const RawRgbaImage(this.bytes, this.width, this.height);

  /// `width * height * 4` bytes, in the order [ui.PixelFormat.rgba8888] packs.
  final Uint8List bytes;
  final int width;
  final int height;

  @override
  Future<RawRgbaImage> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture<RawRgbaImage>(this);

  @override
  ImageStreamCompleter loadImage(
    RawRgbaImage key,
    ImageDecoderCallback decode,
  ) {
    return OneFrameImageStreamCompleter(
      _decode(key),
      informationCollector: () => <DiagnosticsNode>[
        DiagnosticsProperty<ImageProvider<Object>>('Image provider', this),
      ],
    );
  }

  Future<ImageInfo> _decode(RawRgbaImage key) {
    // `decodeImageFromPixels` reports a bad payload asynchronously instead of
    // throwing, leaving the completer (and so the cell) blank forever. Validate
    // first so a malformed image reaches the error builder as a clean error.
    final expected = key.width > 0 && key.height > 0
        ? key.width * key.height * 4
        : -1;
    if (expected <= 0 || key.bytes.length != expected) {
      return Future<ImageInfo>.error(
        FlutterError(
          'RawRgbaImage has ${key.bytes.length} bytes for a '
          '${key.width}x${key.height} image',
        ),
      );
    }
    final completer = Completer<ImageInfo>();
    ui.decodeImageFromPixels(
      key.bytes,
      key.width,
      key.height,
      ui.PixelFormat.rgba8888,
      (ui.Image image) => completer.complete(ImageInfo(image: image)),
    );
    return completer.future;
  }

  // Two providers share a decoded image only when they hold the very same bytes
  // buffer: the buffer is never mutated, so identity is the cheapest sound key.
  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RawRgbaImage &&
          identical(bytes, other.bytes) &&
          width == other.width &&
          height == other.height;

  @override
  int get hashCode => Object.hash(identityHashCode(bytes), width, height);

  @override
  String toString() => 'RawRgbaImage(${width}x$height, ${bytes.length} bytes)';
}
