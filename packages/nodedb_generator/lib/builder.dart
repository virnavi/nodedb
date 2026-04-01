import 'package:build/build.dart';
import 'package:source_gen/source_gen.dart';

import 'src/nodedb_generator.dart';

/// Entry point for the `build_runner` code generator.
///
/// Generates `.nodedb.g.dart` part files for classes annotated with
/// `@collection`, `@node`, or `@edge` from the `nodedb` package.
Builder nodedbBuilder(BuilderOptions options) =>
    SharedPartBuilder(
      [
        CollectionGenerator(),
        NodeGenerator(),
        EdgeGenerator(),
        PreferencesAnnotationGenerator(),
        ViewGenerator(),
        JsonModelGenerator(),
      ],
      'nodedb',
    );
