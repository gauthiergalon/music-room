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
    return NavigationBar(
      selectedIndex: selectedIndex,
      onDestinationSelected: onIndexSelected,
      destinations: const <NavigationDestination>[
        NavigationDestination(icon: Icon(Icons.group_outlined), label: 'Room'),
        NavigationDestination(
          icon: Icon(Icons.search_outlined),
          label: 'Search',
        ),
        NavigationDestination(
          icon: Icon(Icons.person_outline),
          label: 'Profile',
        ),
      ],
      backgroundColor: Theme.of(context).colorScheme.surface,
    );
  }
}
