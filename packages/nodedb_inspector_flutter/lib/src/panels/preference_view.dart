import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Preference panel: preference store key-value browser.
class PreferenceView extends StatefulWidget {
  final PreferencePanel panel;
  const PreferenceView({super.key, required this.panel});

  @override
  State<PreferenceView> createState() => _PreferenceViewState();
}

class _PreferenceViewState extends State<PreferenceView> {
  final _storeController = TextEditingController(text: 'default');
  String _currentStore = 'default';

  @override
  void dispose() {
    _storeController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    Map<String, dynamic> values;
    try {
      values = widget.panel.allValues(_currentStore);
    } catch (_) {
      values = {};
    }

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: [
              const Text('Store: ', style: TextStyle(
                color: InspectorColors.textDim, fontSize: 12)),
              SizedBox(
                width: 200,
                child: TextField(
                  controller: _storeController,
                  style: const TextStyle(fontSize: 12),
                  decoration: const InputDecoration(
                    isDense: true,
                    contentPadding:
                        EdgeInsets.symmetric(horizontal: 8, vertical: 8),
                  ),
                  onSubmitted: (v) =>
                      setState(() => _currentStore = v.trim()),
                ),
              ),
              const SizedBox(width: 8),
              IconButton(
                icon: const Icon(Icons.refresh, size: 18),
                onPressed: () =>
                    setState(() => _currentStore = _storeController.text.trim()),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: values.isEmpty
              ? const EmptyState(message: 'No preferences in this store')
              : ListView(
                  padding: const EdgeInsets.all(16),
                  children: [
                    SectionHeader(title: '$_currentStore (${values.length} keys)'),
                    for (final entry in values.entries)
                      Card(
                        child: Padding(
                          padding: const EdgeInsets.all(12),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(entry.key, style: const TextStyle(
                                color: InspectorColors.accent,
                                fontSize: 12,
                                fontWeight: FontWeight.bold,
                              )),
                              const SizedBox(height: 4),
                              Text(
                                '${entry.value}',
                                style: const TextStyle(
                                  color: InspectorColors.text, fontSize: 12),
                              ),
                            ],
                          ),
                        ),
                      ),
                  ],
                ),
        ),
      ],
    );
  }
}
