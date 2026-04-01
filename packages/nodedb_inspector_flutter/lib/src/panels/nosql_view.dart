import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// NoSQL panel: hierarchical mesh → database → schema → collection browser.
class NoSqlView extends StatefulWidget {
  final List<DatabaseEntry> entries;
  const NoSqlView({super.key, required this.entries});

  @override
  State<NoSqlView> createState() => _NoSqlViewState();
}

class _NoSqlViewState extends State<NoSqlView> {
  String? _selectedMesh;
  String? _selectedDatabase;
  String? _selectedSchema;
  String? _selectedCollection;

  @override
  void initState() {
    super.initState();
    _autoSelect();
  }

  void _autoSelect() {
    final meshes = widget.entries.map((e) => e.mesh).toSet().toList();
    if (meshes.length == 1) {
      _selectedMesh = meshes.first;
      final dbs = _databasesForMesh(_selectedMesh!);
      if (dbs.length == 1) {
        _selectedDatabase = dbs.first;
        final schemas = _schemasForDatabase(_selectedDatabase!);
        if (schemas.length == 1) _selectedSchema = schemas.first;
      }
    }
  }

  List<String> get _meshes =>
      widget.entries.map((e) => e.mesh).toSet().toList()..sort();

  List<String> _databasesForMesh(String mesh) => widget.entries
      .where((e) => e.mesh == mesh)
      .map((e) => e.database)
      .toSet()
      .toList()
    ..sort();

  DatabaseEntry? get _activeEntry {
    if (_selectedMesh == null || _selectedDatabase == null) return null;
    try {
      return widget.entries.firstWhere(
        (e) => e.mesh == _selectedMesh && e.database == _selectedDatabase,
      );
    } catch (_) {
      return null;
    }
  }

  List<String> _schemasForDatabase(String database) {
    final entry = _activeEntry;
    if (entry == null) return [];
    final names = entry.panel.collectionNames();
    final schemas = <String>{};
    for (final name in names) {
      if (name.contains('.')) {
        schemas.add(name.split('.').first);
      } else {
        schemas.add('default');
      }
    }
    return schemas.toList()..sort();
  }

  List<String> _collectionsForSchema(String schema) {
    final entry = _activeEntry;
    if (entry == null) return [];
    final names = entry.panel.collectionNames();
    return names.where((n) {
      if (n.contains('.')) return n.split('.').first == schema;
      return schema == 'default';
    }).toList()
      ..sort();
  }

  @override
  Widget build(BuildContext context) {
    if (widget.entries.isEmpty) {
      return const EmptyState(message: 'No databases');
    }

    return Column(
      children: [
        // Selector bar
        _buildSelectorBar(),
        const Divider(height: 1),
        // Content
        Expanded(child: _buildContent()),
      ],
    );
  }

  Widget _buildSelectorBar() {
    return Container(
      color: InspectorColors.surfaceAlt,
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      child: Wrap(
        spacing: 12,
        runSpacing: 8,
        children: [
          _buildDropdown(
            label: 'Mesh',
            value: _selectedMesh,
            items: _meshes,
            onChanged: (v) => setState(() {
              _selectedMesh = v;
              _selectedDatabase = null;
              _selectedSchema = null;
              _selectedCollection = null;
              // Auto-select if single database
              final dbs = _databasesForMesh(v!);
              if (dbs.length == 1) {
                _selectedDatabase = dbs.first;
                final schemas = _schemasForDatabase(dbs.first);
                if (schemas.length == 1) _selectedSchema = schemas.first;
              }
            }),
          ),
          if (_selectedMesh != null)
            _buildDropdown(
              label: 'Database',
              value: _selectedDatabase,
              items: _databasesForMesh(_selectedMesh!),
              onChanged: (v) => setState(() {
                _selectedDatabase = v;
                _selectedSchema = null;
                _selectedCollection = null;
                final schemas = _schemasForDatabase(v!);
                if (schemas.length == 1) _selectedSchema = schemas.first;
              }),
            ),
          if (_selectedDatabase != null)
            _buildDropdown(
              label: 'Schema',
              value: _selectedSchema,
              items: _schemasForDatabase(_selectedDatabase!),
              onChanged: (v) => setState(() {
                _selectedSchema = v;
                _selectedCollection = null;
              }),
            ),
        ],
      ),
    );
  }

  Widget _buildDropdown({
    required String label,
    required String? value,
    required List<String> items,
    required ValueChanged<String?> onChanged,
  }) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('$label: ', style: const TextStyle(
          color: InspectorColors.textDim, fontSize: 12)),
        DropdownButton<String>(
          value: items.contains(value) ? value : null,
          hint: Text('Select $label', style: const TextStyle(
            color: InspectorColors.textDim, fontSize: 12)),
          isDense: true,
          underline: const SizedBox.shrink(),
          dropdownColor: InspectorColors.surface,
          style: const TextStyle(
            color: InspectorColors.accent, fontSize: 12),
          items: items.map((item) => DropdownMenuItem(
            value: item,
            child: Text(item),
          )).toList(),
          onChanged: onChanged,
        ),
      ],
    );
  }

  Widget _buildContent() {
    if (_selectedSchema == null) {
      return const EmptyState(
        message: 'Select mesh, database, and schema',
        icon: Icons.account_tree,
      );
    }

    final collections = _collectionsForSchema(_selectedSchema!);
    final entry = _activeEntry;
    if (entry == null) return const EmptyState(message: 'No data');

    final stats = entry.panel.collectionStats();

    return Row(
      children: [
        // Collection list
        SizedBox(
          width: 240,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const SectionHeader(title: 'Collections'),
              Expanded(
                child: collections.isEmpty
                    ? const EmptyState(message: 'No collections')
                    : ListView.builder(
                        itemCount: collections.length,
                        itemBuilder: (ctx, i) {
                          final name = collections[i];
                          final count = stats[name] ?? 0;
                          final selected = name == _selectedCollection;
                          return ListTile(
                            dense: true,
                            selected: selected,
                            selectedTileColor: InspectorColors.surface,
                            title: Text(name, style: TextStyle(
                              color: selected
                                  ? InspectorColors.accent
                                  : InspectorColors.text,
                              fontSize: 12,
                            )),
                            trailing: Text('$count', style: const TextStyle(
                              color: InspectorColors.textDim, fontSize: 11)),
                            onTap: () =>
                                setState(() => _selectedCollection = name),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
        const VerticalDivider(width: 1),
        // Document preview
        Expanded(
          child: _selectedCollection == null
              ? const EmptyState(
                  message: 'Select a collection',
                  icon: Icons.touch_app,
                )
              : _DocumentList(
                  panel: entry.panel,
                  collection: _selectedCollection!,
                ),
        ),
      ],
    );
  }
}

class _DocumentList extends StatefulWidget {
  final NoSqlPanel panel;
  final String collection;
  const _DocumentList({required this.panel, required this.collection});

  @override
  State<_DocumentList> createState() => _DocumentListState();
}

class _DocumentListState extends State<_DocumentList> {
  int _offset = 0;
  static const _limit = 20;

  @override
  void didUpdateWidget(_DocumentList old) {
    super.didUpdateWidget(old);
    if (old.collection != widget.collection) _offset = 0;
  }

  @override
  Widget build(BuildContext context) {
    final docs = widget.panel.documentPreview(
      widget.collection,
      limit: _limit,
      offset: _offset,
    );

    return Column(
      children: [
        SectionHeader(
          title: '${widget.collection} (offset $_offset)',
          action: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              IconButton(
                icon: const Icon(Icons.chevron_left, size: 18),
                onPressed: _offset > 0
                    ? () => setState(() => _offset = (_offset - _limit).clamp(0, _offset))
                    : null,
              ),
              IconButton(
                icon: const Icon(Icons.chevron_right, size: 18),
                onPressed: docs.length == _limit
                    ? () => setState(() => _offset += _limit)
                    : null,
              ),
            ],
          ),
        ),
        Expanded(
          child: docs.isEmpty
              ? const EmptyState(message: 'No documents')
              : ListView.builder(
                  itemCount: docs.length,
                  itemBuilder: (ctx, i) {
                    final doc = docs[i];
                    final json = documentToJson(doc);
                    final data = json['data'] as Map<String, dynamic>? ?? {};
                    return ExpansionTile(
                      dense: true,
                      tilePadding: const EdgeInsets.symmetric(horizontal: 16),
                      title: Text(
                        'ID: ${doc.id}',
                        style: const TextStyle(
                          color: InspectorColors.accent, fontSize: 12),
                      ),
                      subtitle: Text(
                        data.keys.take(4).join(', '),
                        style: const TextStyle(
                          color: InspectorColors.textDim, fontSize: 11),
                      ),
                      children: [
                        for (final entry in data.entries)
                          KeyValueRow(
                            label: entry.key,
                            value: '${entry.value}',
                          ),
                        const SizedBox(height: 8),
                      ],
                    );
                  },
                ),
        ),
      ],
    );
  }
}
