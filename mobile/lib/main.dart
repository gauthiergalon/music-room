import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'controllers/room_controller.dart';
import 'controllers/auth_controller.dart';
import 'screens/main_screen.dart';
import 'screens/login_page.dart';
import 'core/theme.dart';

void main() {
  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => AuthController()),
        ChangeNotifierProvider(create: (_) => RoomController()),
      ],
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
      // Consumer va réagir dès que notifyListeners() est appelé dans AuthController
      home: Consumer<AuthController>(
        builder: (context, auth, _) {
          if (auth.isAuthenticated) {
            return const MainScreen();
          } else {
            return const LoginPage(); // Remplacer par ta page de connexion
          }
        },
      ),
    );
  }
}
