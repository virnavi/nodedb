# nodedb_flutter_libs

Pre-compiled NodeDB native binaries for Flutter. This FFI plugin package bundles the compiled Rust library for all supported platforms (Android, iOS, macOS, Linux, Windows).

Part of the [NodeDB](https://github.com/mshakib/nodedb) monorepo.

## Installation

```yaml
dependencies:
  nodedb_flutter_libs:
    path: ../nodedb_flutter_libs
```

## Supported Platforms

| Platform | Architecture |
|----------|-------------|
| Android | arm64-v8a, armeabi-v7a, x86_64 |
| iOS | arm64, simulator (arm64, x86_64) |
| macOS | arm64, x86_64 |
| Linux | x86_64 |
| Windows | x86_64 |

## Building

Use the build script to compile for all targets:

```bash
./scripts/build_targets.sh
```

Requires Rust toolchain, Android NDK (for Android), and Xcode (for iOS/macOS).
