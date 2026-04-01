import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:test/test.dart';

class _Person {
  final String name;
  final int age;
  _Person(this.name, this.age);
}

class _Animal {
  final String species;
  _Animal(this.species);
}

void main() {
  group('SerializerRegistry', () {
    late SerializerRegistry registry;

    setUp(() {
      registry = SerializerRegistry();
    });

    test('register and serialize', () {
      registry.register<_Person>(
        (p) => {'name': p.name, 'age': p.age},
      );

      final result = registry.serialize(_Person('Alice', 30));
      expect(result, {'name': 'Alice', 'age': 30});
    });

    test('serialize returns null for unregistered type', () {
      expect(registry.serialize(_Animal('cat')), isNull);
    });

    test('has returns true for registered type', () {
      registry.register<_Person>((p) => {'name': p.name});
      expect(registry.has<_Person>(), isTrue);
      expect(registry.has<_Animal>(), isFalse);
    });

    test('hasForValue checks runtime type', () {
      registry.register<_Person>((p) => {'name': p.name});
      expect(registry.hasForValue(_Person('Bob', 25)), isTrue);
      expect(registry.hasForValue(_Animal('dog')), isFalse);
    });

    test('length tracks registered serializers', () {
      expect(registry.length, 0);
      registry.register<_Person>((p) => {'name': p.name});
      expect(registry.length, 1);
      registry.register<_Animal>((a) => {'species': a.species});
      expect(registry.length, 2);
    });
  });

  group('JsonSerializable', () {
    test('mixin contract', () {
      final obj = _SerializableThing('test');
      expect(obj.toJson(), {'value': 'test'});
    });
  });
}

class _SerializableThing with JsonSerializable {
  final String value;
  _SerializableThing(this.value);

  @override
  Map<String, dynamic> toJson() => {'value': value};
}
