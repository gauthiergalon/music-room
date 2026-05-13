import 'dart:async';

import 'package:flutter/material.dart';
import 'package:palette_generator/palette_generator.dart';

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
    final track = _audioService.currentTrack;
    final imageUrl = track?.imageUrl;

    if (imageUrl == null || imageUrl.isEmpty) {
      _updateSeedColor(_defaultSeedColor);
      return;
    }

    _extractColorFromImage(imageUrl);
  }

  void setTrackColor(String? imageUrl) {
    if (imageUrl == null || imageUrl.isEmpty) {
      _updateSeedColor(_defaultSeedColor);
      return;
    }

    _extractColorFromImage(imageUrl);
  }

  Future<void> _extractColorFromImage(String imageUrl) async {
    try {
      final imageProvider = NetworkImage(imageUrl);
      final paletteGenerator = await PaletteGenerator.fromImageProvider(
        imageProvider,
        maximumColorCount: 8,
        timeout: const Duration(seconds: 5),
      );

      final dominantColor =
          paletteGenerator.dominantColor?.color ??
          paletteGenerator.vibrantColor?.color ??
          paletteGenerator.mutedColor?.color ??
          _defaultSeedColor;

      _updateSeedColor(dominantColor);
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
