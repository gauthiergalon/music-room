import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../services/app_settings_service.dart';

Future<void> showBatteryOptimizationDialog(BuildContext context) async {
  if (!Platform.isAndroid) return;

  final settings = await AppSettingsService.getInstance();

  if (settings.batteryOptimizationDismissed) {
    return;
  }

  if (!context.mounted) return;

  await showDialog<void>(
    context: context,
    barrierDismissible: false,
    builder: (ctx) => AlertDialog(
      title: const Text('Battery Optimization'),
      content: const Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'To ensure uninterrupted music playback, please disable battery optimization for Music Room.',
          ),
          SizedBox(height: 16),
          Text(
            'This prevents the app from being killed in the background while playing music.',
            style: TextStyle(fontSize: 14, color: Colors.grey),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () {
            settings.setBatteryOptimizationDismissed(true);
            Navigator.of(ctx).pop();
          },
          child: const Text('Don\'t show again'),
        ),
        FilledButton(
          onPressed: () {
            Navigator.of(ctx).pop();
            _openBatterySettings();
          },
          child: const Text('Open Settings'),
        ),
      ],
    ),
  );
}

void _openBatterySettings() {
  if (Platform.isAndroid) {
    try {
      const platform = MethodChannel('com.music_room/battery');
      platform.invokeMethod('openBatteryOptimizationSettings');
    } catch (_) {
      // Fallback: try to open general settings
      SystemNavigator.pop();
    }
  }
}
