import 'dart:async';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:app_links/app_links.dart';
import 'package:just_audio_background/just_audio_background.dart';

import 'controllers/room_controller.dart';
import 'controllers/auth_controller.dart';
import 'controllers/friends_controller.dart';
import 'screens/main_screen.dart';
import 'screens/login_page.dart';
import 'screens/reset_password_page.dart';
import 'core/theme.dart';
import 'core/utils/ui_utils.dart';

final GlobalKey<NavigatorState> navigatorKey = GlobalKey<NavigatorState>();

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  
  await JustAudioBackground.init(
    androidNotificationChannelId: 'com.music_room.bg_audio.channel.audio',
    androidNotificationChannelName: 'Music Room Playback',
    androidNotificationOngoing: true,
  );

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => AuthController()),
        ChangeNotifierProvider(create: (_) => RoomController()),
        ChangeNotifierProvider(create: (_) => FriendsController()),
      ],
      child: const MyApp(),
    ),
  );
}

class MyApp extends StatefulWidget {
  const MyApp({super.key});

  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {
  final _appLinks = AppLinks();
  StreamSubscription<Uri>? _linkSubscription;

  @override
  void initState() {
    super.initState();
    _initDeepLinks();
  }

  @override
  void dispose() {
    _linkSubscription?.cancel();
    super.dispose();
  }

  void _initDeepLinks() {
    _linkSubscription = _appLinks.uriLinkStream.listen(
      (uri) async {
        debugPrint('Received Deep Link: $uri');

        final token = uri.queryParameters['token'];

        if (uri.host == 'confirm-email' && token != null) {
          _handleConfirmEmail(token);
        } else if (uri.host == 'reset-password' && token != null) {
          _handleResetPassword(token);
        }
      },
      onError: (err) {
        debugPrint('Deep Link Error: $err');
      },
    );
  }

  Future<void> _handleConfirmEmail(String token) async {
    final currentContext = navigatorKey.currentContext;
    if (currentContext == null) return;

    try {
      await currentContext.read<AuthController>().confirmEmail(token);
      if (currentContext.mounted) {
        UiUtils.showSuccess(
          currentContext,
          'Your email has been successfully verified!',
        );
      }
    } catch (e) {
      if (currentContext.mounted) {
        UiUtils.showError(currentContext, 'Failed to verify email');
      }
    }
  }

  void _handleResetPassword(String token) {
    final currentContext = navigatorKey.currentContext;
    if (currentContext == null) return;

    Navigator.of(currentContext).push(
      MaterialPageRoute(builder: (context) => ResetPasswordPage(token: token)),
    );
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Music Room',
      theme: AppTheme.darkTheme,
      navigatorKey: navigatorKey,
      home: Consumer<AuthController>(
        builder: (context, auth, _) {
          if (auth.isAuthenticated) {
            return const MainScreen();
          } else {
            return const LoginPage();
          }
        },
      ),
    );
  }
}
