# nodedb_ffi

Low-level `dart:ffi` bindings for the NodeDB native library. Provides raw C-interop functions (`openRaw`, `executeRaw`, `writeTxnRaw`) used internally by the `nodedb` package.

Part of the [NodeDB](https://github.com/mshakib/nodedb) monorepo.

## Installation

```yaml
dependencies:
  nodedb_ffi:
    path: ../nodedb_ffi
```

## Usage

This package is used internally by `nodedb`. Direct usage is not recommended — use the `nodedb` package for typed access.

```dart
import 'package:nodedb_ffi/nodedb_ffi.dart';

final bindings = NodeDbBindings(dynamicLibrary);
final handle = bindings.openRaw(directory);
final result = bindings.executeRaw(handle, action, payload);
```
