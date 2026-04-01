import 'dart:ffi';
import 'dart:io';

/// Loads the NodeDB native library for the current platform.
DynamicLibrary loadNodeDbLibrary() {
  if (Platform.isAndroid) return DynamicLibrary.open('libnodedb_ffi.so');
  if (Platform.isIOS) return DynamicLibrary.process();
  if (Platform.isMacOS) {
    return DynamicLibrary.open('libnodedb_ffi.dylib');
  }
  if (Platform.isLinux) return DynamicLibrary.open('libnodedb_ffi.so');
  if (Platform.isWindows) return DynamicLibrary.open('nodedb_ffi.dll');
  throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
}

/// Allows overriding the library path for testing.
DynamicLibrary loadNodeDbLibraryFromPath(String path) {
  return DynamicLibrary.open(path);
}
