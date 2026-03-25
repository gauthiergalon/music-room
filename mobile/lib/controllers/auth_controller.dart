import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../core/network/api_client.dart';
import '../core/exceptions/api_exception.dart';
import '../models/user.dart';

class AuthController extends ChangeNotifier {
  bool _isAuthenticated = false;
  String? _token;
  User? _user;

  bool get isAuthenticated => _isAuthenticated;
  String? get token => _token;
  User? get user => _user;

  AuthController() {
    ApiClient.onUnauthorized = () => logout();
    _loadToken();
  }

  Future<void> _loadToken() async {
    final prefs = await SharedPreferences.getInstance();
    _token = prefs.getString('jwt_token');

    if (_token != null) {
      _isAuthenticated = true;
      notifyListeners();
      await fetchUserInfo();
    }
  }

  Future<void> fetchUserInfo() async {
    if (_token == null) return;
    try {
      final data = await ApiClient.get('/users/me');
      if (data != null) {
        _user = User(
          id: data['id'],
          username: data['username'],
          email: data['email'],
          emailConfirmed: data['email_confirmed'],
          googleId: data['google_id'],
        );
        notifyListeners();
      }
    } on ApiException catch (e) {
      if (e.statusCode == 401) {
        await logout();
      }
      debugPrint('Failed to fetch user info: $e');
    } catch (e) {
      debugPrint('Unexpected error fetching user info: $e');
    }
  }

  Future<void> updateUsername(String newUsername) async {
    final data = await ApiClient.patch(
      '/users/me/username',
      body: {'username': newUsername},
    );
    if (data != null) {
      _user = _user!.copyWith(username: data['username']);
      notifyListeners();
    }
  }

  Future<void> updateEmail(String newEmail) async {
    final data = await ApiClient.patch(
      '/users/me/email',
      body: {'new_email': newEmail},
    );
    if (data != null) {
      _user = _user!.copyWith(email: data['email'], emailConfirmed: false);
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

  Future<void> sendEmailConfirmation() async {
    await ApiClient.post('/users/me/send-confirmation-email');
  }

  Future<void> register(String username, String email, String password) async {
    final data = await ApiClient.post(
      '/auth/register',
      body: {'username': username, 'email': email, 'password': password},
    );

    if (data != null) {
      _token = data['access_token'];
      _isAuthenticated = true;

      final prefs = await SharedPreferences.getInstance();
      await prefs.setString('jwt_token', _token!);
      if (data['refresh_token'] != null) {
        await prefs.setString('refresh_token', data['refresh_token']);
      }

      notifyListeners();
      await fetchUserInfo();
    }
  }

  Future<void> login(String email, String password) async {
    final data = await ApiClient.post(
      '/auth/login',
      body: {'email': email, 'password': password},
    );

    if (data != null) {
      _token = data['access_token'];
      _isAuthenticated = true;

      final prefs = await SharedPreferences.getInstance();
      await prefs.setString('jwt_token', _token!);
      if (data['refresh_token'] != null) {
        await prefs.setString('refresh_token', data['refresh_token']);
      }

      notifyListeners();
      await fetchUserInfo();
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
    throw ApiException('Google login is not implemented yet');
  }

  Future<void> logout() async {
    final prefs = await SharedPreferences.getInstance();
    final refreshToken = prefs.getString('refresh_token');

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

    await prefs.remove('jwt_token');
    await prefs.remove('refresh_token');

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
