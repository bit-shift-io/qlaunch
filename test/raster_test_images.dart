import 'dart:ui' as ui;

ui.Image image(int width, int height) {
  final recorder = ui.PictureRecorder();
  ui.Canvas(recorder).drawColor(const ui.Color(0xff123456), ui.BlendMode.src);
  final picture = recorder.endRecording();
  final result = picture.toImageSync(width, height);
  picture.dispose();
  return result;
}
