import 'package:flutter/material.dart';
import 'room_page.dart';
import 'search_page.dart';
import 'profile_page.dart';
import '../widgets/app_navbar.dart';

class MainScreen extends StatefulWidget {
  const MainScreen({super.key});

  @override
  State<MainScreen> createState() => _MainScreenState();
}

class _MainScreenState extends State<MainScreen> {
  int _selectedIndex = 0;

  @override
  Widget build(BuildContext context) {
    final pages = <Widget>[
      const RoomPage(),
      const SearchPage(),
      const ProfilePage(),
    ];

    return Scaffold(
      body: pages[_selectedIndex],
      bottomNavigationBar: AppNavBar(
        selectedIndex: _selectedIndex,
        onIndexSelected: (int index) => setState(() => _selectedIndex = index),
      ),
    );
  }
}
