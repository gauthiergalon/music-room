import 'package:flutter/foundation.dart';
import 'package:flutter_dotenv/flutter_dotenv.dart';
import 'package:google_sign_in/google_sign_in.dart';

import '../core/exceptions/api_exception.dart';
import '../core/logger/logger.dart';
import '../core/network/api_client.dart';
import '../core/storage/session_storage.dart';
import '../models/user.dart';

class AuthController extends ChangeNotifier {
  bool _isAuthenticated = false;
  bool _isLoadingUser = false;
  bool _isLoggingOut = false;
  User? _user;
  Future<void>? _googleInit;

  bool get isAuthenticated => _isAuthenticated;
  bool get isLoadingUser => _isLoadingUser;
  User? get user => _user;

  /// Called at the start of every logout, before the session is cleared, so
  /// the rest of the app can drop what belongs to the user (open room,
  /// friends list...). Must be synchronous.
  VoidCallback? onLoggedOut;

  AuthController() {
    ApiClient.onUnauthorized = _handleUnauthorized;
    _loadSession();
  }

  @override
  void dispose() {
    if (ApiClient.onUnauthorized == _handleUnauthorized) {
      ApiClient.onUnauthorized = null;
    }
    super.dispose();
  }

  /// The session could not be refreshed: the refresh token is no longer
  /// valid, so there is nothing to revoke on the server.
  void _handleUnauthorized() {
    _performLogout(sendServerLogout: false);
  }

  Future<void> _performLogout({bool sendServerLogout = true}) async {
    if (_isLoggingOut || !_isAuthenticated) return;
    _isLoggingOut = true;

    try {
      onLoggedOut?.call();

      _isAuthenticated = false;
      _user = null;
      notifyListeners();

      final refreshToken = await SessionStorage.getRefreshToken();
      await SessionStorage.clear();

      if (sendServerLogout && refreshToken != null) {
        try {
          await ApiClient.post(
            '/auth/logout',
            body: {'refresh_token': refreshToken},
          );
        } catch (e) {
          // Best effort: the local session is gone either way.
          logger.debug('Server logout failed');
        }
      }
    } finally {
      _isLoggingOut = false;
    }
  }

  Future<void> _loadSession() async {
    final token = await SessionStorage.getAccessToken();
    if (token == null) return;

    _isAuthenticated = true;
    notifyListeners();
    await fetchUserInfo();
  }

  Future<void> _storeAuthenticatedSession(Map<String, dynamic> data) async {
    await _saveTokens(data);
    _isAuthenticated = true;
    notifyListeners();
    await fetchUserInfo();
  }

  Future<void> _saveTokens(Map<String, dynamic> data) async {
    final accessToken = data['access_token']?.toString();
    if (accessToken == null || accessToken.isEmpty) {
      throw ApiException('Authentication response is missing an access token.');
    }

    await SessionStorage.saveSession(
      accessToken: accessToken,
      refreshToken: data['refresh_token']?.toString(),
    );
  }

  void _setUserFrom(dynamic data) {
    if (data is! Map) return;
    _user = User.fromJson(Map<String, dynamic>.from(data));
    notifyListeners();
  }

  Future<void> fetchUserInfo() async {
    if (!_isAuthenticated) return;
    try {
      _isLoadingUser = true;
      notifyListeners();

      final data = await ApiClient.get('/users/me');
      if (data is Map) {
        _user = User.fromJson(Map<String, dynamic>.from(data));
      }
    } on ApiException catch (e) {
      logger.error('Failed to fetch user info', error: e);
      if (e.statusCode == 401) {
        await logout();
      } else if (e.statusCode == 404) {
        // The account no longer exists on the server.
        logger.warning('User no longer exists, forcing logout');
        await _performLogout(sendServerLogout: false);
      }
    } catch (e) {
      logger.error('Unexpected error fetching user info', error: e);
    } finally {
      _isLoadingUser = false;
      notifyListeners();
    }
  }

  // ------------------------------------------------------------- account

  Future<void> register(String username, String email, String password) async {
    final data = await ApiClient.post(
      '/auth/register',
      body: {'username': username, 'email': email, 'password': password},
    );
    await _storeAuthenticatedSession(Map<String, dynamic>.from(data as Map));
  }

  Future<void> login(String email, String password) async {
    final data = await ApiClient.post(
      '/auth/login',
      body: {'email': email, 'password': password},
    );
    await _storeAuthenticatedSession(Map<String, dynamic>.from(data as Map));
  }

  Future<void> logout() => _performLogout(sendServerLogout: true);

  Future<void> forgotPassword(String email) async {
    await ApiClient.post('/auth/forgot-password', body: {'email': email});
  }

  Future<void> resetPassword(String token, String newPassword) async {
    await ApiClient.post(
      '/auth/reset-password',
      body: {'token': token, 'new_password': newPassword},
    );
  }

  // -------------------------------------------------------------- Google

  String? get _googleClientId {
    final value = dotenv.env['GOOGLE_CLIENT_ID']?.trim();
    return value == null || value.isEmpty ? null : value;
  }

  /// Initializes Google sign-in, once. On the web the client ID is the app's
  /// own; elsewhere it is passed as the server's. Either way the ID tokens
  /// are issued for that client, the audience the backend expects.
  Future<void> ensureGoogleInitialized() async {
    final clientId = _googleClientId;
    if (clientId == null) {
      throw ApiException('Google sign-in is not configured.');
    }

    try {
      await (_googleInit ??= GoogleSignIn.instance.initialize(
        clientId: kIsWeb ? clientId : null,
        serverClientId: kIsWeb ? null : clientId,
      ));
    } catch (e) {
      _googleInit = null;
      rethrow;
    }
  }

  /// ID tokens of the sign-ins made with Google's own button, the only way
  /// to sign in with Google on the web. Failures arrive as stream errors.
  Stream<String> get googleButtonIdTokens => GoogleSignIn
      .instance
      .authenticationEvents
      .map(
        (event) => switch (event) {
          GoogleSignInAuthenticationEventSignIn(:final user) =>
            user.authentication.idToken,
          GoogleSignInAuthenticationEventSignOut() => null,
        },
      )
      .where((idToken) => idToken != null)
      .cast<String>();

  /// Runs the Google sign-in flow and returns the Google ID token. Not
  /// available on the web, where the token comes from Google's button.
  Future<String> _googleIdToken() async {
    if (kIsWeb) {
      throw ApiException('Use the Google button to sign in with Google.');
    }

    await ensureGoogleInitialized();
    final account = await GoogleSignIn.instance.authenticate(
      scopeHint: ['email', 'profile'],
    );
    final idToken = account.authentication.idToken;
    if (idToken == null) {
      throw ApiException('Could not get a token from Google.');
    }
    return idToken;
  }

  /// Logs in with Google. [idToken] is given when Google's button already
  /// produced it (web); otherwise the sign-in flow runs first.
  Future<void> loginWithGoogle({String? idToken}) async {
    try {
      final token = idToken ?? await _googleIdToken();
      final data = await ApiClient.post(
        '/auth/google-login',
        body: {'id_token': token},
      );
      await _storeAuthenticatedSession(Map<String, dynamic>.from(data as Map));
    } on ApiException {
      rethrow;
    } catch (e) {
      logger.error('Google sign-in failed', error: e);
      throw ApiException('Google sign-in failed.');
    }
  }

  /// Links a Google account to the account that is currently logged in.
  /// [idToken] as for [loginWithGoogle].
  Future<void> linkGoogleAccount({String? idToken}) async {
    try {
      final token = idToken ?? await _googleIdToken();
      final data = await ApiClient.post(
        '/users/me/google',
        body: {'id_token': token},
      );
      _setUserFrom(data);
    } on ApiException {
      rethrow;
    } catch (e) {
      logger.error('Linking the Google account failed', error: e);
      throw ApiException('Could not link the Google account.');
    }
  }

  // ------------------------------------------------------------- profile

  Future<void> updateUsername(String newUsername) async {
    final data = await ApiClient.patch(
      '/users/me/username',
      body: {'username': newUsername},
    );
    _setUserFrom(data);
  }

  /// Asks for an email change. The address only changes once the link sent
  /// to [newEmail] has been confirmed.
  Future<void> updateEmail(String newEmail) async {
    await ApiClient.patch('/users/me/email', body: {'new_email': newEmail});
  }

  /// Changes the password. The server revokes every other session and
  /// returns new tokens for this one.
  Future<void> updatePassword(
    String currentPassword,
    String newPassword,
  ) async {
    final data = await ApiClient.patch(
      '/users/me/password',
      body: {'current_password': currentPassword, 'new_password': newPassword},
    );
    if (data is Map && data['access_token'] != null) {
      await _saveTokens(Map<String, dynamic>.from(data));
    }
  }

  Future<void> updateFavoriteGenres(List<String> favoriteGenres) async {
    final data = await ApiClient.patch(
      '/users/me/favorite-genres',
      body: {'favorite_genres': favoriteGenres},
    );
    _setUserFrom(data);
  }

  Future<void> updatePrivacyLevel(String privacyLevel) async {
    final data = await ApiClient.patch(
      '/users/me/privacy',
      body: {'privacy_level': privacyLevel},
    );
    _setUserFrom(data);
  }

  Future<void> sendEmailConfirmation() async {
    await ApiClient.post('/users/me/send-confirmation-email');
  }

  /// Confirms an email address (current or new) with the token of the link.
  /// Works without a session; a logged-in user's profile is refreshed after.
  Future<void> confirmEmail(String token) async {
    await ApiClient.patch('/auth/confirm-email', body: {'token': token});
    if (_isAuthenticated) await fetchUserInfo();
  }

  Future<void> enableSubscription() async {
    final data = await ApiClient.post('/users/me/subscription');
    _setUserFrom(data);
  }
}
