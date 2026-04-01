/// Describes a single action that a panel supports.
class PanelAction {
  /// Action name used in the command protocol (e.g., 'collectionStats').
  final String name;

  /// Human-readable description of what this action does.
  final String? description;

  /// Parameter descriptors for this action.
  final List<PanelActionParam> params;

  const PanelAction({
    required this.name,
    this.description,
    this.params = const [],
  });

  Map<String, dynamic> toJson() => {
        'name': name,
        if (description != null) 'description': description,
        if (params.isNotEmpty)
          'params': params.map((p) => p.toJson()).toList(),
      };
}

/// Describes a parameter for a panel action.
class PanelActionParam {
  /// Parameter name.
  final String name;

  /// Type hint: 'string', 'int', 'double', 'bool', 'map', 'list'.
  final String type;

  /// Whether this parameter is required.
  final bool required;

  /// Default value when not provided.
  final dynamic defaultValue;

  /// Human-readable description.
  final String? description;

  const PanelActionParam({
    required this.name,
    required this.type,
    this.required = false,
    this.defaultValue,
    this.description,
  });

  Map<String, dynamic> toJson() => {
        'name': name,
        'type': type,
        'required': required,
        if (defaultValue != null) 'defaultValue': defaultValue,
        if (description != null) 'description': description,
      };
}
