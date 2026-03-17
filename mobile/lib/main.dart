import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'controllers/room_controller.dart';
import 'screens/main_screen.dart';
import 'core/theme.dart';

void main() {
  runApp(
    ChangeNotifierProvider(
      create: (_) => RoomController(),
      child: const MyApp(),
    ),
  );
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Music Room',
      theme: AppTheme.darkTheme,
      home: const MainScreen(),
    );
  }
}
