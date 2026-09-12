import 'package:flutter/material.dart';

class AppNavBar extends StatelessWidget {
  final int selectedIndex;
  final void Function(int) onIndexSelected;

  const AppNavBar({
    super.key,
    required this.selectedIndex,
    required this.onIndexSelected,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return NavigationBar(
      selectedIndex: selectedIndex,
      onDestinationSelected: onIndexSelected,
      indicatorColor: theme.colorScheme.primary,
      labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
      destinations: [
        NavigationDestination(
          icon: Icon(
            selectedIndex == 0 ? Icons.group : Icons.group_outlined,
          ),
          label: 'Room',
        ),
        NavigationDestination(
          icon: Icon(
            selectedIndex == 1 ? Icons.search : Icons.search_outlined,
          ),
          label: 'Search',
        ),
        NavigationDestination(
          icon: Icon(
            selectedIndex == 2 ? Icons.person : Icons.person_outline,
          ),
          label: 'Profile',
        ),
      ],
      backgroundColor: theme.colorScheme.surface,
    );
  }
}
