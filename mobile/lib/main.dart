import 'dart:async';

import 'package:flutter/material.dart';
import 'package:app_links/app_links.dart';
import 'package:just_audio_background/just_audio_background.dart';
import 'package:provider/provider.dart';

import 'core/logger/logger.dart';
import 'core/theme.dart';
import 'core/utils/ui_utils.dart';
import 'controllers/auth_controller.dart';
import 'controllers/friends_controller.dart';
import 'controllers/room_controller.dart';
import 'controllers/theme_controller.dart';
import 'services/audio_service.dart';
import 'screens/login_page.dart';
import 'screens/main_screen.dart';
import 'screens/reset_password_page.dart';

final GlobalKey<NavigatorState> navigatorKey = GlobalKey<NavigatorState>();

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  logger.init();

  await JustAudioBackground.init(
    androidNotificationChannelId: 'com.music_room.bg_audio.channel.audio',
    androidNotificationChannelName: 'Music Room Playback',
    androidNotificationOngoing: true,
  );

  logger.info('App started');

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => AuthController()),
        ChangeNotifierProvider(create: (_) => AudioService()),
        ChangeNotifierProxyProvider<AudioService, RoomController>(
          create: (context) => RoomController(audioService: context.read<AudioService>()),
          update: (_, audio, previous) => previous ?? RoomController(audioService: audio),
        ),
        ChangeNotifierProvider(create: (_) => FriendsController()),
        ChangeNotifierProxyProvider<AudioService, ThemeController>(
          create: (context) => ThemeController(context.read<AudioService>()),
          update: (_, audio, previous) => previous ?? ThemeController(audio),
        ),
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
      _handleIncomingUri,
      onError: (err) => logger.error('Deep Link Error', error: err),
    );
  }

  void _handleIncomingUri(Uri uri) {
    logger.debug('Received Deep Link: $uri');

    final token = uri.queryParameters['token'];
    if (token == null) return;

    switch (uri.host) {
      case 'confirm-email':
        _handleConfirmEmail(token);
        break;
      case 'reset-password':
        _handleResetPassword(token);
        break;
    }
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
    return Consumer<ThemeController>(
      builder: (context, themeController, _) {
        return MaterialApp(
          title: 'Music Room',
          theme: AppTheme.darkThemeWithSeed(themeController.seedColor),
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
      },
    );
  }
}