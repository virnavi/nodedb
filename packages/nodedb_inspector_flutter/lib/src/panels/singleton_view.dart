import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Singleton panel: singleton collection browser.
class SingletonView extends StatefulWidget {
  final SingletonPanel panel;
  const SingletonView({super.key, required this.panel});

  @override
  State<SingletonView> createState() => _SingletonViewState();
}

class _SingletonViewState extends State<SingletonView> {
  String? _selected;

  @override
  Widget build(BuildContext context) {
    final names = widget.panel.singletonNames();
    final summary = widget.panel.summary();

    return Row(
      children: [
        SizedBox(
          width: 240,
          child: Column(
            children: [
              SectionHeader(title: 'Singletons (${summary['count'] ?? 0})'),
              Expanded(
                child: names.isEmpty
                    ? const EmptyState(message: 'No singletons')
                    : ListView.builder(
                        itemCount: names.length,
                        itemBuilder: (ctx, i) {
                          final name = names[i];
                          return ListTile(
                            dense: true,
                            selected: name == _selected,
                            selectedTileColor: InspectorColors.surface,
                            title: Text(name, style: TextStyle(
                              color: name == _selected
                                  ? InspectorColors.accent
                                  : InspectorColors.text,
                              fontSize: 12,
                            )),
                            onTap: () => setState(() => _selected = name),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
        const VerticalDivider(width: 1),
        Expanded(
          child: _selected == null
              ? const EmptyState(
                  message: 'Select a singleton', icon: Icons.touch_app)
              : _singletonDetail(),
        ),
      ],
    );
  }

  Widget _singletonDetail() {
    try {
      final doc = widget.panel.singletonData(_selected!);
      final json = documentToJson(doc);
      final data = json['data'] as Map<String, dynamic>? ?? {};
      return ListView(
        padding: const EdgeInsets.all(16),
        children: [
          SectionHeader(title: _selected!),
          KeyValueRow(label: 'ID', value: '${json['id']}'),
          const SizedBox(height: 8),
          for (final entry in data.entries)
            KeyValueRow(label: entry.key, value: '${entry.value}'),
        ],
      );
    } catch (e) {
      return EmptyState(message: 'Error: $e');
    }
  }
}
