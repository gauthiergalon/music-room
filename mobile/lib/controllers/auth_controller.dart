import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../core/network/api_client.dart';
import '../core/exceptions/api_exception.dart';

class AuthController extends ChangeNotifier {
  bool _isAuthenticated = false;
  String? _token;
  String? _username;
  String? _email;

  bool get isAuthenticated => _isAuthenticated;
  String? get token => _token;
  String get username => _username ?? 'User';
  String get email => _email ?? 'user@example.com';

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
        _username = data['username'];
        _email = data['email'];
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
      _username = data['username'];
      notifyListeners();
    }
  }

  Future<void> updateEmail(String newEmail) async {
    final data = await ApiClient.patch(
      '/users/me/email',
      body: {'new_email': newEmail},
    );
    if (data != null) {
      _email = data['email'];
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
    _username = null;
    _email = null;

    await prefs.remove('jwt_token');
    await prefs.remove('refresh_token');

    notifyListeners();
  }
}
