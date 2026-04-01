import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:nodedb_inspector_flutter/nodedb_inspector_flutter.dart';

void main() {
  // Full widget tests with NodeDB require native library loading which
  // isn't available in the test environment. These tests validate the
  // theme configuration and reusable helper widgets.

  group('InspectorColors', () {
    test('defines all 12 colors', () {
      final colors = [
        InspectorColors.bg,
        InspectorColors.surface,
        InspectorColors.surfaceAlt,
        InspectorColors.border,
        InspectorColors.text,
        InspectorColors.textDim,
        InspectorColors.accent,
        InspectorColors.green,
        InspectorColors.red,
        InspectorColors.yellow,
        InspectorColors.magenta,
        InspectorColors.cyan,
      ];
      expect(colors, hasLength(12));
      for (final c in colors) {
        expect(c, isA<Color>());
      }
    });

    test('bg is dark', () {
      expect(InspectorColors.bg.computeLuminance(), lessThan(0.1));
    });

    test('text is light', () {
      expect(InspectorColors.text.computeLuminance(), greaterThan(0.4));
    });

    test('surface is darker than text', () {
      expect(InspectorColors.surface.computeLuminance(),
          lessThan(InspectorColors.text.computeLuminance()));
    });
  });

  group('inspectorTheme', () {
    test('returns dark theme', () {
      final theme = inspectorTheme();
      expect(theme.brightness, Brightness.dark);
    });

    test('scaffold background is bg color', () {
      final theme = inspectorTheme();
      expect(theme.scaffoldBackgroundColor, InspectorColors.bg);
    });

    test('color scheme primary is accent', () {
      final theme = inspectorTheme();
      expect(theme.colorScheme.primary, InspectorColors.accent);
    });

    test('color scheme secondary is cyan', () {
      final theme = inspectorTheme();
      expect(theme.colorScheme.secondary, InspectorColors.cyan);
    });

    test('color scheme error is red', () {
      final theme = inspectorTheme();
      expect(theme.colorScheme.error, InspectorColors.red);
    });

    test('text theme uses monospace', () {
      final theme = inspectorTheme();
      expect(theme.textTheme.bodyMedium?.fontFamily, 'monospace');
      expect(theme.textTheme.bodyLarge?.fontFamily, 'monospace');
      expect(theme.textTheme.titleLarge?.fontFamily, 'monospace');
      expect(theme.textTheme.labelLarge?.fontFamily, 'monospace');
    });

    test('card theme has zero elevation', () {
      final theme = inspectorTheme();
      expect(theme.cardTheme.elevation, 0);
      expect(theme.cardTheme.color, InspectorColors.surface);
    });

    test('navigation rail has surfaceAlt background', () {
      final theme = inspectorTheme();
      expect(theme.navigationRailTheme.backgroundColor,
          InspectorColors.surfaceAlt);
    });

    test('app bar has surfaceAlt background', () {
      final theme = inspectorTheme();
      expect(theme.appBarTheme.backgroundColor, InspectorColors.surfaceAlt);
      expect(theme.appBarTheme.foregroundColor, InspectorColors.text);
      expect(theme.appBarTheme.elevation, 0);
    });

    test('divider uses border color', () {
      final theme = inspectorTheme();
      expect(theme.dividerColor, InspectorColors.border);
    });
  });

  group('MetricCard', () {
    testWidgets('displays label and value', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: MetricCard(label: 'Test Label', value: '42'),
          ),
        ),
      );

      expect(find.text('Test Label'), findsOneWidget);
      expect(find.text('42'), findsOneWidget);
    });

    testWidgets('shows icon when provided', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: MetricCard(
              label: 'With Icon',
              value: '7',
              icon: Icons.storage,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.storage), findsOneWidget);
    });

    testWidgets('applies custom value color', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: MetricCard(
              label: 'Colored',
              value: '99',
              valueColor: Colors.green,
            ),
          ),
        ),
      );

      final text = tester.widget<Text>(find.text('99'));
      expect(text.style?.color, Colors.green);
    });

    testWidgets('default color is accent', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: MetricCard(label: 'Default', value: '5'),
          ),
        ),
      );

      final text = tester.widget<Text>(find.text('5'));
      expect(text.style?.color, InspectorColors.accent);
    });
  });

  group('SectionHeader', () {
    testWidgets('displays title', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SectionHeader(title: 'My Section'),
          ),
        ),
      );

      expect(find.text('My Section'), findsOneWidget);
    });

    testWidgets('shows action widget', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SectionHeader(
              title: 'With Action',
              action: IconButton(
                icon: const Icon(Icons.refresh),
                onPressed: () {},
              ),
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.refresh), findsOneWidget);
    });

    testWidgets('no action renders without error', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SectionHeader(title: 'No Action'),
          ),
        ),
      );

      expect(find.text('No Action'), findsOneWidget);
    });
  });

  group('KeyValueRow', () {
    testWidgets('displays label and value', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: KeyValueRow(label: 'Name', value: 'Alice'),
          ),
        ),
      );

      expect(find.text('Name'), findsOneWidget);
      expect(find.text('Alice'), findsOneWidget);
    });

    testWidgets('label has fixed width', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: KeyValueRow(label: 'Key', value: 'Val'),
          ),
        ),
      );

      final sizedBox = tester.widget<SizedBox>(
        find.ancestor(
          of: find.text('Key'),
          matching: find.byType(SizedBox),
        ),
      );
      expect(sizedBox.width, 160);
    });
  });

  group('InspectorDataTable', () {
    testWidgets('renders columns and rows', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SingleChildScrollView(
              child: InspectorDataTable(
                columns: ['ID', 'Name', 'Age'],
                rows: [
                  ['1', 'Alice', '30'],
                  ['2', 'Bob', '25'],
                ],
              ),
            ),
          ),
        ),
      );

      expect(find.text('ID'), findsOneWidget);
      expect(find.text('Name'), findsOneWidget);
      expect(find.text('Age'), findsOneWidget);
      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('Bob'), findsOneWidget);
    });

    testWidgets('handles empty rows', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: InspectorDataTable(
              columns: ['Col'],
              rows: [],
            ),
          ),
        ),
      );

      expect(find.text('Col'), findsOneWidget);
    });
  });

  group('EmptyState', () {
    testWidgets('shows message and default icon', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: EmptyState(message: 'Nothing here'),
          ),
        ),
      );

      expect(find.text('Nothing here'), findsOneWidget);
      expect(find.byIcon(Icons.inbox), findsOneWidget);
    });

    testWidgets('shows custom icon', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: EmptyState(
              message: 'Touch',
              icon: Icons.touch_app,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.touch_app), findsOneWidget);
    });

    testWidgets('contains icon and text', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: EmptyState(message: 'Centered'),
          ),
        ),
      );

      // EmptyState renders both icon and message text
      expect(find.byType(EmptyState), findsOneWidget);
      expect(find.text('Centered'), findsOneWidget);
      expect(find.byType(Icon), findsOneWidget);
    });
  });
}
