# NodeDB FFI Build Status

Version: `0.0.2`

## Platforms

| Platform | Rust Target | Output | Status | Notes |
|---|---|---|---|---|
| Windows x64 | `x86_64-pc-windows-msvc` | `packages/nodedb_flutter_libs/windows/nodedb_ffi.dll` | N/A | Requires Windows host |
| Android arm64 | `aarch64-linux-android` | `packages/nodedb_flutter_libs/android/src/main/jniLibs/arm64-v8a/libnodedb_ffi.so` | ✅ Built | NDK: 28.2.13676358 |
| Android armv7 | `armv7-linux-androideabi` | `packages/nodedb_flutter_libs/android/src/main/jniLibs/armeabi-v7a/libnodedb_ffi.so` | ✅ Built | NDK: 28.2.13676358 |
| Android x86_64 | `x86_64-linux-android` | `packages/nodedb_flutter_libs/android/src/main/jniLibs/x86_64/libnodedb_ffi.so` | ✅ Built | NDK: 28.2.13676358 |
| Linux x64 | `x86_64-unknown-linux-gnu` | `packages/nodedb_flutter_libs/linux/libnodedb_ffi.so` | N/A | Requires Linux host |
| iOS device | `aarch64-apple-ios` | `packages/nodedb_flutter_libs/ios/nodedb.xcframework` | ✅ Built | Xcode, static lib (.a) |
| iOS simulator | `aarch64-apple-ios-sim` | `packages/nodedb_flutter_libs/ios/nodedb.xcframework` | ✅ Built | Xcode, static lib (.a) |
| macOS arm64 | `aarch64-apple-darwin` | `packages/nodedb_flutter_libs/macos/libnodedb_ffi.dylib` | ✅ Built | Universal binary via lipo |
| macOS x64 | `x86_64-apple-darwin` | `packages/nodedb_flutter_libs/macos/libnodedb_ffi.dylib` | ✅ Built | Universal binary via lipo |

## Build Instructions

### Prerequisites

```
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin x86_64-apple-darwin
rustup target add x86_64-unknown-linux-gnu
```

### Windows (run on Windows)

```powershell
cd rust
cargo build --release --target x86_64-pc-windows-msvc -p nodedb-ffi
copy target\release\nodedb_ffi.dll ..\packages\nodedb_flutter_libs\windows\
```

### Android (run on Windows/macOS/Linux with NDK)

Set `ANDROID_NDK_HOME` to your NDK path, then:

```bash
# NDK path (Windows): D:\env\android\sdk\ndk\28.2.13676358
# NDK path (macOS):   ~/Library/Android/sdk/ndk/<version>
export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/<host-tag>/bin:$PATH"

cd rust
cargo build --release --target aarch64-linux-android   -p nodedb-ffi
cargo build --release --target armv7-linux-androideabi -p nodedb-ffi
cargo build --release --target x86_64-linux-android    -p nodedb-ffi

# Copy outputs
cp target/aarch64-linux-android/release/libnodedb_ffi.so   ../packages/nodedb_flutter_libs/android/src/main/jniLibs/arm64-v8a/
cp target/armv7-linux-androideabi/release/libnodedb_ffi.so ../packages/nodedb_flutter_libs/android/src/main/jniLibs/armeabi-v7a/
cp target/x86_64-linux-android/release/libnodedb_ffi.so    ../packages/nodedb_flutter_libs/android/src/main/jniLibs/x86_64/
```

### Linux (run on Linux)

```bash
cd rust
cargo build --release --target x86_64-unknown-linux-gnu -p nodedb-ffi
cp target/x86_64-unknown-linux-gnu/release/libnodedb_ffi.so ../packages/nodedb_flutter_libs/linux/
```

### iOS & macOS (run on macOS with Xcode)

```bash
cd rust
# macOS universal binary
cargo build --release --target aarch64-apple-darwin -p nodedb-ffi
cargo build --release --target x86_64-apple-darwin  -p nodedb-ffi
lipo -create \
  target/aarch64-apple-darwin/release/libnodedb_ffi.dylib \
  target/x86_64-apple-darwin/release/libnodedb_ffi.dylib \
  -output ../packages/nodedb_flutter_libs/macos/libnodedb_ffi.dylib

# iOS xcframework
cargo build --release --target aarch64-apple-ios     -p nodedb-ffi
cargo build --release --target aarch64-apple-ios-sim -p nodedb-ffi
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libnodedb_ffi.a \
  -library target/aarch64-apple-ios-sim/release/libnodedb_ffi.a \
  -output ../packages/nodedb_flutter_libs/ios/nodedb.xcframework
```

## Environment (Windows build machine)

- Rust toolchain: stable
- NDK: `28.2.13676358` at `D:\env\android\sdk\ndk\`
- CMake: `3.22.1` at `D:\env\android\sdk\cmake\`
- Config: `rust/.cargo/config.toml`
