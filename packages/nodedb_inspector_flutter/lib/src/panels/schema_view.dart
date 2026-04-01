import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Schema panel: overview of schemas and collections.
class SchemaView extends StatefulWidget {
  final SchemaPanel panel;
  const SchemaView({super.key, required this.panel});

  @override
  State<SchemaView> createState() => _SchemaViewState();
}

class _SchemaViewState extends State<SchemaView> {
  String? _selectedCollection;

  @override
  Widget build(BuildContext context) {
    final overview = widget.panel.overview();
    final schemas = overview['schemas'] as List? ?? [];
    final fingerprint = overview['fingerprint'] as String? ?? '';

    return Row(
      children: [
        SizedBox(
          width: 260,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SectionHeader(title: 'Schemas (${schemas.length})'),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Text(
                  'Fingerprint: ${fingerprint.length > 16 ? '${fingerprint.substring(0, 16)}...' : fingerprint}',
                  style: const TextStyle(
                    color: InspectorColors.textDim, fontSize: 10),
                ),
              ),
              const SizedBox(height: 8),
              Expanded(
                child: schemas.isEmpty
                    ? const EmptyState(message: 'No schemas')
                    : ListView(
                        children: [
                          for (final schema in schemas)
                            if (schema is Map<String, dynamic>)
                              _schemaSection(schema),
                        ],
                      ),
              ),
            ],
          ),
        ),
        const VerticalDivider(width: 1),
        Expanded(
          child: _selectedCollection == null
              ? const EmptyState(
                  message: 'Select a collection', icon: Icons.touch_app)
              : _collectionDetail(),
        ),
      ],
    );
  }

  Widget _schemaSection(Map<String, dynamic> schema) {
    final name = schema['name'] as String? ?? 'unknown';
    final collections = schema['collections'] as List? ?? [];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
          child: Text(
            name,
            style: const TextStyle(
              color: InspectorColors.magenta,
              fontSize: 12,
              fontWeight: FontWeight.bold,
            ),
          ),
        ),
        for (final col in collections)
          ListTile(
            dense: true,
            title: Text('$col',
                style: TextStyle(
                  color: _selectedCollection == col
                      ? InspectorColors.accent
                      : InspectorColors.text,
                  fontSize: 12,
                )),
            selected: _selectedCollection == col,
            selectedTileColor: InspectorColors.surface,
            onTap: () => setState(() => _selectedCollection = '$col'),
          ),
      ],
    );
  }

  Widget _collectionDetail() {
    final detail = widget.panel.collectionDetail(_selectedCollection!);
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        SectionHeader(title: _selectedCollection!),
        for (final entry in detail.entries)
          KeyValueRow(label: entry.key, value: '${entry.value}'),
      ],
    );
  }
}
