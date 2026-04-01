import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:test/test.dart';

class _TestSource implements InspectorDataSource {
  final DataSourceDescriptor _descriptor;
  final bool _connected;

  _TestSource({
    required String id,
    String displayName = 'Test',
    bool connected = true,
  })  : _descriptor = DataSourceDescriptor(
          id: id,
          displayName: displayName,
        ),
        _connected = connected;

  @override
  DataSourceDescriptor get descriptor => _descriptor;

  @override
  bool get isConnected => _connected;

  @override
  dynamic query(Map<String, dynamic> params) => {'result': 'ok'};

  @override
  Map<String, dynamic> stats() => {'connected': _connected};
}

void main() {
  group('DataSourceRegistry', () {
    late DataSourceRegistry registry;

    setUp(() {
      registry = DataSourceRegistry();
    });

    test('register and retrieve data source', () {
      final source = _TestSource(id: 'metrics');
      registry.register(source);

      expect(registry.length, 1);
      expect(registry.contains('metrics'), isTrue);
      expect(registry.get('metrics'), same(source));
    });

    test('duplicate registration throws StateError', () {
      registry.register(_TestSource(id: 'metrics'));
      expect(
        () => registry.register(_TestSource(id: 'metrics')),
        throwsStateError,
      );
    });

    test('unregister removes data source', () {
      registry.register(_TestSource(id: 'metrics'));
      expect(registry.unregister('metrics'), isTrue);
      expect(registry.contains('metrics'), isFalse);
      expect(registry.length, 0);
    });

    test('unregister returns false for unknown ID', () {
      expect(registry.unregister('nonexistent'), isFalse);
    });

    test('get returns null for unknown ID', () {
      expect(registry.get('nonexistent'), isNull);
    });

    test('getAs returns typed data source', () {
      final source = _TestSource(id: 'metrics');
      registry.register(source);

      final result = registry.getAs<_TestSource>('metrics');
      expect(result, same(source));
    });

    test('all returns all registered sources', () {
      registry.register(_TestSource(id: 'a'));
      registry.register(_TestSource(id: 'b'));
      expect(registry.all.length, 2);
    });

    test('connected filters to connected sources only', () {
      registry.register(_TestSource(id: 'on', connected: true));
      registry.register(_TestSource(id: 'off', connected: false));

      expect(registry.connected.length, 1);
      expect(registry.connected.first.descriptor.id, 'on');
    });
  });
}
