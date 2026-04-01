/// Mixin for objects that can serialize themselves to JSON-compatible maps.
mixin JsonSerializable {
  Map<String, dynamic> toJson();
}

/// Type alias for a function that converts a domain object to JSON.
typedef JsonSerializer<T> = Map<String, dynamic> Function(T value);
