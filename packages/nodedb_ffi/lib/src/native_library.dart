import 'dart:ffi';
import 'dart:io';

/// Loads the NodeDB native library for the current platform.
DynamicLibrary loadNodeDbLibrary() {
  if (Platform.isAndroid) return DynamicLibrary.open('libnodedb_ffi.so');
  if (Platform.isIOS) return DynamicLibrary.process();
  if (Platform.isMacOS) {
    // Try loading from common locations
    const paths = [
      'libnodedb_ffi.dylib',
      '../rust/target/debug/libnodedb_ffi.dylib',
      '../rust/target/release/libnodedb_ffi.dylib',
      '../../rust/target/debug/libnodedb_ffi.dylib',
      '../../rust/target/release/libnodedb_ffi.dylib',
    ];
    for (final path in paths) {
      try {
        return DynamicLibrary.open(path);
      } catch (_) {
        continue;
      }
    }
    throw UnsupportedError(
      'Could not find libnodedb_ffi.dylib. '
      'Build with: cd rust && cargo build',
    );
  }
  if (Platform.isLinux) return DynamicLibrary.open('libnodedb_ffi.so');
  if (Platform.isWindows) return DynamicLibrary.open('nodedb_ffi.dll');
  throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
}

/// Allows overriding the library path for testing.
DynamicLibrary loadNodeDbLibraryFromPath(String path) {
  return DynamicLibrary.open(path);
}
