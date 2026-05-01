import 'package:flutter/material.dart';
import 'package:google_sign_in/google_sign_in.dart';

import '../core/exceptions/api_exception.dart';
import '../core/network/api_client.dart';
import '../core/storage/session_storage.dart';
import '../models/user.dart';

class AuthController extends ChangeNotifier {
  bool _isAuthenticated = false;
  bool _isLoadingUser = false;
  String? _token;
  User? _user;

  bool get isAuthenticated => _isAuthenticated;
  bool get isLoadingUser => _isLoadingUser;
  String? get token => _token;
  User? get user => _user;

  AuthController() {
    GoogleSignIn.instance.initialize(
      serverClientId:
          '1068662764722-kfnc69v1mk1aq8gsb6e8h3kh1kl287qf.apps.googleusercontent.com',
    );
    ApiClient.onUnauthorized = logout;
    _loadSession();
  }

  Future<void> _loadSession() async {
    _token = await SessionStorage.getAccessToken();

    if (_token != null) {
      _isAuthenticated = true;
      notifyListeners();
      await fetchUserInfo();
    }
  }

  List<String>? _parseFavoriteGenres(dynamic rawFavoriteGenres) {
    if (rawFavoriteGenres is! List) {
      return null;
    }

    final favoriteGenres = rawFavoriteGenres
        .map((item) => item.toString())
        .where((item) => item.isNotEmpty)
        .toList();

    return favoriteGenres.isEmpty ? null : favoriteGenres;
  }

  User _buildUser(Map<String, dynamic> data) {
    return User(
      id: data['id'],
      username: data['username'],
      email: data['email'],
      emailConfirmed: data['email_confirmed'],
      googleId: data['google_id'],
      favoriteGenres: _parseFavoriteGenres(data['favorite_genres']),
      privacyLevel: data['privacy_level']?.toString() ?? 'Friends',
    );
  }

  Future<void> _storeAuthenticatedSession(Map<String, dynamic> data) async {
    final accessToken = data['access_token']?.toString();
    if (accessToken == null || accessToken.isEmpty) {
      throw ApiException('Authentication response is missing an access token.');
    }

    _token = accessToken;
    _isAuthenticated = true;

    await SessionStorage.saveSession(
      accessToken: accessToken,
      refreshToken: data['refresh_token']?.toString(),
    );

    notifyListeners();
    await fetchUserInfo();
  }

  Future<Map<String, dynamic>> _authenticateWithGoogle() async {
    final GoogleSignInAccount account = await GoogleSignIn.instance
        .authenticate(scopeHint: ['email', 'profile']);

    final GoogleSignInAuthentication auth = account.authentication;
    final String? idToken = auth.idToken;

    if (idToken == null) {
      throw ApiException('Échec de la récupération du token Google.');
    }

    final response = await ApiClient.post(
      '/auth/google-login',
      body: {'id_token': idToken},
    );

    return Map<String, dynamic>.from(response as Map);
  }

  Future<void> fetchUserInfo() async {
    if (_token == null) return;
    try {
      _isLoadingUser = true;
      notifyListeners();

      final data = await ApiClient.get('/users/me');
      if (data != null) {
        _user = _buildUser(Map<String, dynamic>.from(data as Map));
      }
    } on ApiException catch (e) {
      if (e.statusCode == 401) {
        await logout();
      }
      debugPrint('Failed to fetch user info: $e');
    } catch (e) {
      debugPrint('Unexpected error fetching user info: $e');
    } finally {
      _isLoadingUser = false;
      notifyListeners();
    }
  }

  Future<void> updateUsername(String newUsername) async {
    final data = await ApiClient.patch(
      '/users/me/username',
      body: {'username': newUsername},
    );
    if (data != null) {
      _user = _user?.copyWith(username: data['username']);
      notifyListeners();
    }
  }

  Future<void> updateEmail(String newEmail) async {
    final data = await ApiClient.patch(
      '/users/me/email',
      body: {'new_email': newEmail},
    );
    if (data != null) {
      _user = _user?.copyWith(email: data['email'], emailConfirmed: false);
      notifyListeners();
    }
  }

  Future<void> updatePassword(
    String currentPassword,
    String newPassword,
  ) async {
    await ApiClient.patch(
      '/users/me/password',
      body: {'current_password': currentPassword, 'new_password': newPassword},
    );
  }

  Future<void> updateFavoriteGenres(List<String>? favoriteGenres) async {
    final data = await ApiClient.patch(
      '/users/me/favorite-genres',
      body: {'favorite_genres': favoriteGenres},
    );

    if (data != null) {
      _user = _user?.copyWith(
        favoriteGenres: _parseFavoriteGenres(data['favorite_genres']),
      );
      notifyListeners();
    }
  }

  Future<void> updatePrivacyLevel(String privacyLevel) async {
    final data = await ApiClient.patch(
      '/users/me/privacy',
      body: {'privacy_level': privacyLevel},
    );

    if (data != null) {
      _user = _user?.copyWith(
        privacyLevel: data['privacy_level']?.toString() ?? privacyLevel,
      );
      notifyListeners();
    }
  }

  Future<void> sendEmailConfirmation() async {
    await ApiClient.post('/users/me/send-confirmation-email');
  }

  Future<void> register(String username, String email, String password) async {
    final data = await ApiClient.post(
      '/auth/register',
      body: {'username': username, 'email': email, 'password': password},
    );

    if (data != null) {
      await _storeAuthenticatedSession(Map<String, dynamic>.from(data as Map));
    }
  }

  Future<void> login(String email, String password) async {
    final data = await ApiClient.post(
      '/auth/login',
      body: {'email': email, 'password': password},
    );

    if (data != null) {
      await _storeAuthenticatedSession(Map<String, dynamic>.from(data as Map));
    }
  }

  Future<void> forgotPassword(String email) async {
    await ApiClient.post('/auth/forgot-password', body: {'email': email});
  }

  Future<void> resetPassword(String token, String newPassword) async {
    await ApiClient.post(
      '/auth/reset-password',
      body: {'token': token, 'new_password': newPassword},
    );
  }

  Future<void> loginWithGoogle() async {
    try {
      final data = await _authenticateWithGoogle();
      await _storeAuthenticatedSession(data);
    } catch (e) {
      debugPrint('Erreur lors de la connexion Google: $e');
      throw ApiException('La connexion Google a échoué: $e');
    }
  }

  Future<void> linkGoogleAccount() async {
    try {
      final data = await _authenticateWithGoogle();
      await _storeAuthenticatedSession(data);
    } catch (e) {
      debugPrint('Erreur lors de la liaison du compte Google: $e');
      throw ApiException('Google link failed');
    }
  }

  Future<void> logout() async {
    final refreshToken = await SessionStorage.getRefreshToken();

    if (_token != null && refreshToken != null) {
      try {
        await ApiClient.post(
          '/auth/logout',
          body: {'refresh_token': refreshToken},
        );
      } catch (_) {
        // Ignore logout errors
      }
    }

    _isAuthenticated = false;
    _token = null;
    _user = null;

    await SessionStorage.clear();

    notifyListeners();
  }

  Future<void> confirmEmail(String token) async {
    try {
      await ApiClient.patch('/users/me/confirm-email?token=$token');

      _user = _user?.copyWith(emailConfirmed: true);
      notifyListeners();
    } catch (e) {
      throw Exception('Failed to verify email: $e');
    }
  }
}
