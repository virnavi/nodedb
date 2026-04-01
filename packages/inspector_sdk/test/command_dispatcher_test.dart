import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:test/test.dart';

class _TestPanel implements InspectorPanel {
  final PanelDescriptor _descriptor;
  final bool _available;

  _TestPanel({
    required String id,
    bool available = true,
    int sortOrder = 100,
  })  : _descriptor = PanelDescriptor(
          id: id,
          displayName: id,
          sortOrder: sortOrder,
          actions: [
            const PanelAction(name: 'stats'),
            const PanelAction(name: 'echo'),
          ],
        ),
        _available = available;

  @override
  PanelDescriptor get descriptor => _descriptor;

  @override
  bool get isAvailable => _available;

  @override
  Map<String, dynamic> summary() => {'panel': _descriptor.id};

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'summary':
        return summary();
      case 'stats':
        return {'count': 42};
      case 'echo':
        return params['value'];
      default:
        throw ArgumentError('Unknown action: $action');
    }
  }

  @override
  void onRegister() {}
  @override
  void onUnregister() {}
}

void main() {
  group('CommandDispatcher', () {
    late PanelRegistry registry;
    late CommandDispatcher dispatcher;

    setUp(() {
      registry = PanelRegistry();
      dispatcher = CommandDispatcher(registry);
    });

    test('enabledPanels returns available panel IDs', () {
      registry.register(_TestPanel(id: 'a', sortOrder: 10));
      registry.register(_TestPanel(id: 'b', available: false, sortOrder: 20));

      final result = dispatcher.dispatch('enabledPanels', {});
      expect(result, ['a']);
    });

    test('panelDescriptors returns descriptors for all panels', () {
      registry.register(_TestPanel(id: 'test'));

      final result = dispatcher.dispatch('panelDescriptors', {})
          as List<Map<String, dynamic>>;
      expect(result.length, 1);
      expect(result.first['id'], 'test');
    });

    test('panel command dispatches to correct panel', () {
      registry.register(_TestPanel(id: 'myPanel'));

      final result = dispatcher.dispatch('panel', {
        'panel': 'myPanel',
        'action': 'stats',
      });
      expect(result, {'count': 42});
    });

    test('panel command passes params to panel dispatch', () {
      registry.register(_TestPanel(id: 'myPanel'));

      final result = dispatcher.dispatch('panel', {
        'panel': 'myPanel',
        'action': 'echo',
        'value': 'hello',
      });
      expect(result, 'hello');
    });

    test('panel command returns disabled for unavailable panel', () {
      registry.register(_TestPanel(id: 'off', available: false));

      final result = dispatcher.dispatch('panel', {
        'panel': 'off',
        'action': 'stats',
      }) as Map<String, dynamic>;
      expect(result['error'], 'panel_disabled');
      expect(result['panel'], 'off');
    });

    test('panel command throws for unknown panel', () {
      expect(
        () => dispatcher.dispatch('panel', {
          'panel': 'nonexistent',
          'action': 'stats',
        }),
        throwsArgumentError,
      );
    });

    test('panel command throws when panel or action missing', () {
      expect(
        () => dispatcher.dispatch('panel', {}),
        throwsArgumentError,
      );
      expect(
        () => dispatcher.dispatch('panel', {'panel': 'x'}),
        throwsArgumentError,
      );
    });

    test('unknown command throws ArgumentError', () {
      expect(
        () => dispatcher.dispatch('unknown', {}),
        throwsArgumentError,
      );
    });

    test('registerCommand adds custom command', () {
      dispatcher.registerCommand('ping', (_) => 'pong');
      expect(dispatcher.dispatch('ping', {}), 'pong');
    });

    test('custom command receives params', () {
      dispatcher.registerCommand(
        'greet',
        (p) => 'Hello, ${p['name']}!',
      );
      expect(
        dispatcher.dispatch('greet', {'name': 'World'}),
        'Hello, World!',
      );
    });
  });
}
