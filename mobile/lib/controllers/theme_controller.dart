import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../services/audio_service.dart';

class ThemeController extends ChangeNotifier {
  final AudioService _audioService;
  Color _seedColor = Colors.deepPurple;

  static const Color _defaultSeedColor = Colors.deepPurple;

  Color get seedColor => _seedColor;

  ThemeController(this._audioService) {
    _audioService.addListener(_onTrackChanged);
  }

  void _onTrackChanged() {
    if (kIsWeb) return;

    final track = _audioService.currentTrack;
    final imageUrl = track?.imageUrl;

    if (imageUrl == null || imageUrl.isEmpty) {
      _updateSeedColor(_defaultSeedColor);
      return;
    }

    _extractColorFromImage(imageUrl);
  }

  void setTrackColor(String? imageUrl) {
    if (kIsWeb) return;

    if (imageUrl == null || imageUrl.isEmpty) {
      _updateSeedColor(_defaultSeedColor);
      return;
    }

    _extractColorFromImage(imageUrl);
  }

  Future<void> _extractColorFromImage(String imageUrl) async {
    try {
      final colorScheme = await ColorScheme.fromImageProvider(
        provider: NetworkImage(imageUrl),
        brightness: Brightness.dark,
      );
      _updateSeedColor(colorScheme.primary);
    } catch (e) {
      _updateSeedColor(_defaultSeedColor);
    }
  }

  void _updateSeedColor(Color color) {
    if (_seedColor != color) {
      _seedColor = color;
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _audioService.removeListener(_onTrackChanged);
    super.dispose();
  }
}
