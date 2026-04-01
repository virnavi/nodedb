import 'package:flutter/material.dart';
import '../inspector_theme.dart';

/// A metric card showing a label and value.
class MetricCard extends StatelessWidget {
  final String label;
  final String value;
  final Color? valueColor;
  final IconData? icon;

  const MetricCard({
    super.key,
    required this.label,
    required this.value,
    this.valueColor,
    this.icon,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                if (icon != null) ...[
                  Icon(icon, size: 14, color: InspectorColors.textDim),
                  const SizedBox(width: 6),
                ],
                Text(label, style: const TextStyle(
                  color: InspectorColors.textDim, fontSize: 11)),
              ],
            ),
            const SizedBox(height: 8),
            Text(value, style: TextStyle(
              color: valueColor ?? InspectorColors.accent,
              fontSize: 24,
              fontWeight: FontWeight.bold,
            )),
          ],
        ),
      ),
    );
  }
}

/// A section header with optional action button.
class SectionHeader extends StatelessWidget {
  final String title;
  final Widget? action;

  const SectionHeader({super.key, required this.title, this.action});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
      child: Row(
        children: [
          Expanded(
            child: Text(title, style: const TextStyle(
              color: InspectorColors.text,
              fontSize: 14,
              fontWeight: FontWeight.bold,
            ), overflow: TextOverflow.ellipsis),
          ),
          if (action != null) action!,
        ],
      ),
    );
  }
}

/// A simple key-value row for detail displays.
class KeyValueRow extends StatelessWidget {
  final String label;
  final String value;

  const KeyValueRow({super.key, required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 160,
            child: Text(label, style: const TextStyle(
              color: InspectorColors.textDim, fontSize: 12)),
          ),
          Expanded(
            child: Text(value, style: const TextStyle(
              color: InspectorColors.text, fontSize: 12)),
          ),
        ],
      ),
    );
  }
}

/// A data table styled for the inspector.
class InspectorDataTable extends StatelessWidget {
  final List<String> columns;
  final List<List<String>> rows;

  const InspectorDataTable({
    super.key,
    required this.columns,
    required this.rows,
  });

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: DataTable(
        headingRowColor: WidgetStateProperty.all(InspectorColors.surfaceAlt),
        dataRowColor: WidgetStateProperty.all(Colors.transparent),
        columns: [
          for (final col in columns)
            DataColumn(label: Text(col, style: const TextStyle(
              color: InspectorColors.accent, fontSize: 11,
              fontWeight: FontWeight.bold))),
        ],
        rows: [
          for (final row in rows)
            DataRow(cells: [
              for (final cell in row)
                DataCell(Text(cell, style: const TextStyle(
                  color: InspectorColors.text, fontSize: 12))),
            ]),
        ],
      ),
    );
  }
}

/// Empty state placeholder.
class EmptyState extends StatelessWidget {
  final String message;
  final IconData icon;

  const EmptyState({
    super.key,
    required this.message,
    this.icon = Icons.inbox,
  });

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 48, color: InspectorColors.textDim),
          const SizedBox(height: 12),
          Text(message, style: const TextStyle(
            color: InspectorColors.textDim, fontSize: 13)),
        ],
      ),
    );
  }
}
