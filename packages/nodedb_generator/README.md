# nodedb_generator

Code generator for NodeDB annotations. Uses `build_runner` and `source_gen` to produce schemas, serialization functions, typed query builders (filter extensions), and DAO classes from annotated model classes.

Part of the [NodeDB](https://github.com/mshakib/nodedb) monorepo.

## Installation

```yaml
dev_dependencies:
  nodedb_generator:
    path: ../nodedb_generator
  build_runner: ^2.4.0
```

## Usage

1. Annotate your models:

```dart
import 'package:nodedb/nodedb.dart';

part 'models.g.dart';

@collection
class User {
  String id = '';
  late String name;
  @Index(unique: true)
  late String email;

  User({this.id = '', required this.name, required this.email});
}
```

2. Run the generator:

```bash
dart run build_runner build --delete-conflicting-outputs
```

This generates `models.g.dart` containing `UserDao`, `UserFilterExtension`, serialization functions, and schema definitions.
