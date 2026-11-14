import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../services/audio_service.dart';

/// Derives the app's seed colour from the cover of the current track.
class ThemeController extends ChangeNotifier {
  final AudioService _audioService;
  Color _seedColor = _defaultSeedColor;

  /// Cover the current colour was computed from (or is being computed from).
  String? _imageUrl;
  bool _disposed = false;

  static const Color _defaultSeedColor = Colors.deepPurple;

  Color get seedColor => _seedColor;

  ThemeController(this._audioService) {
    _audioService.addListener(_onTrackChanged);
  }

  void _onTrackChanged() {
    if (kIsWeb) return;

    final imageUrl = _audioService.currentTrack?.imageUrl;
    final normalized = imageUrl == null || imageUrl.isEmpty ? null : imageUrl;
    if (normalized == _imageUrl) return;
    _imageUrl = normalized;

    if (normalized == null) {
      _updateSeedColor(_defaultSeedColor);
    } else {
      _extractColorFromImage(normalized);
    }
  }

  Future<void> _extractColorFromImage(String imageUrl) async {
    Color color;
    try {
      final colorScheme = await ColorScheme.fromImageProvider(
        provider: NetworkImage(imageUrl),
        brightness: Brightness.dark,
      );
      color = colorScheme.primary;
    } catch (e) {
      color = _defaultSeedColor;
    }

    // The track changed again while this cover was being analysed.
    if (_disposed || imageUrl != _imageUrl) return;
    _updateSeedColor(color);
  }

  void _updateSeedColor(Color color) {
    if (_seedColor != color) {
      _seedColor = color;
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _audioService.removeListener(_onTrackChanged);
    super.dispose();
  }
}
