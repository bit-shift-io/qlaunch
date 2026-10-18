import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/api/api.dart' as rust;
import 'package:qlaunch/src/rust/frb_generated.dart';

void main() {
  setUpAll(() async {
    await RustLib.init(
      externalLibrary: await loadExternalLibrary(
        const ExternalLibraryLoaderConfig(
          stem: 'qlaunch_core',
          ioDirectory: 'rust/target/release/',
          webPrefix: 'pkg/',
          wasmBindgenName: 'wasm_bindgen',
        ),
      ),
    );
  });

  test('core reports its version', () async {
    expect(await rust.version(), startsWith('0.1.'));
  });
}
