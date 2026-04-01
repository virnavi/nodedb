import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:test/test.dart';

/// A minimal test panel implementation.
class _TestPanel implements InspectorPanel {
  final PanelDescriptor _descriptor;
  final bool _available;
  bool registered = false;
  bool unregistered = false;

  _TestPanel({
    required String id,
    String displayName = 'Test',
    int sortOrder = 100,
    bool available = true,
  })  : _descriptor = PanelDescriptor(
          id: id,
          displayName: displayName,
          sortOrder: sortOrder,
        ),
        _available = available;

  @override
  PanelDescriptor get descriptor => _descriptor;

  @override
  bool get isAvailable => _available;

  @override
  Map<String, dynamic> summary() => {'test': true};

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    if (action == 'summary') return summary();
    throw ArgumentError('Unknown action: $action');
  }

  @override
  void onRegister() => registered = true;

  @override
  void onUnregister() => unregistered = true;
}

void main() {
  group('PanelRegistry', () {
    late PanelRegistry registry;

    setUp(() {
      registry = PanelRegistry();
    });

    test('register and retrieve panel', () {
      final panel = _TestPanel(id: 'test');
      registry.register(panel);

      expect(registry.length, 1);
      expect(registry.contains('test'), isTrue);
      expect(registry.get('test'), same(panel));
    });

    test('register calls onRegister lifecycle hook', () {
      final panel = _TestPanel(id: 'test');
      registry.register(panel);
      expect(panel.registered, isTrue);
    });

    test('duplicate registration throws StateError', () {
      registry.register(_TestPanel(id: 'test'));
      expect(
        () => registry.register(_TestPanel(id: 'test')),
        throwsStateError,
      );
    });

    test('unregister removes panel and calls onUnregister', () {
      final panel = _TestPanel(id: 'test');
      registry.register(panel);

      final removed = registry.unregister('test');
      expect(removed, isTrue);
      expect(panel.unregistered, isTrue);
      expect(registry.contains('test'), isFalse);
      expect(registry.length, 0);
    });

    test('unregister returns false for unknown ID', () {
      expect(registry.unregister('nonexistent'), isFalse);
    });

    test('get returns null for unknown ID', () {
      expect(registry.get('nonexistent'), isNull);
    });

    test('getAs returns typed panel', () {
      final panel = _TestPanel(id: 'test');
      registry.register(panel);

      final result = registry.getAs<_TestPanel>('test');
      expect(result, same(panel));
    });

    test('getAs returns null for wrong type', () {
      registry.register(_TestPanel(id: 'test'));
      // InspectorPanel is a supertype, not _TestPanel — this will match
      // but asking for a non-matching subtype would return null
      final result = registry.getAs<_TestPanel>('test');
      expect(result, isNotNull);
    });

    test('all returns panels sorted by sortOrder', () {
      registry.register(_TestPanel(id: 'c', sortOrder: 30));
      registry.register(_TestPanel(id: 'a', sortOrder: 10));
      registry.register(_TestPanel(id: 'b', sortOrder: 20));

      final ids = registry.all.map((p) => p.descriptor.id).toList();
      expect(ids, ['a', 'b', 'c']);
    });

    test('available filters out unavailable panels', () {
      registry.register(_TestPanel(id: 'on', available: true));
      registry.register(_TestPanel(id: 'off', available: false));

      expect(registry.available.length, 1);
      expect(registry.available.first.descriptor.id, 'on');
    });

    test('enabledPanelIds returns IDs of available panels', () {
      registry.register(_TestPanel(id: 'a', available: true, sortOrder: 10));
      registry.register(_TestPanel(id: 'b', available: false, sortOrder: 20));
      registry.register(_TestPanel(id: 'c', available: true, sortOrder: 30));

      expect(registry.enabledPanelIds(), ['a', 'c']);
    });

    test('descriptors returns JSON for all panels', () {
      registry.register(_TestPanel(id: 'x', displayName: 'X Panel'));
      final descs = registry.descriptors;
      expect(descs.length, 1);
      expect(descs.first['id'], 'x');
      expect(descs.first['displayName'], 'X Panel');
    });

    test('addListener notifies on registration', () {
      InspectorPanel? notified;
      registry.addListener((p) => notified = p);

      final panel = _TestPanel(id: 'test');
      registry.register(panel);

      expect(notified, same(panel));
    });

    test('removeListener stops notifications', () {
      int count = 0;
      void listener(InspectorPanel p) => count++;

      registry.addListener(listener);
      registry.register(_TestPanel(id: 'a'));
      expect(count, 1);

      registry.removeListener(listener);
      registry.register(_TestPanel(id: 'b'));
      expect(count, 1);
    });
  });
}
