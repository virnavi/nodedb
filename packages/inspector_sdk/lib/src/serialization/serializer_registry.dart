import 'json_serializable.dart';

/// Registry for type-specific JSON serializers.
///
/// Allows panels to register serializers for their domain types so the
/// command protocol can automatically serialize results.
class SerializerRegistry {
  final Map<Type, JsonSerializer<dynamic>> _serializers = {};

  /// Register a serializer for type [T].
  void register<T>(JsonSerializer<T> serializer) {
    _serializers[T] = (dynamic value) => serializer(value as T);
  }

  /// Serialize a value if a serializer is registered for its runtime type.
  /// Returns null if no serializer is found.
  Map<String, dynamic>? serialize(dynamic value) {
    final serializer = _serializers[value.runtimeType];
    if (serializer == null) return null;
    return serializer(value);
  }

  /// Whether a serializer is registered for type [T].
  bool has<T>() => _serializers.containsKey(T);

  /// Whether a serializer is registered for the given value's runtime type.
  bool hasForValue(dynamic value) =>
      _serializers.containsKey(value.runtimeType);

  /// Number of registered serializers.
  int get length => _serializers.length;
}
