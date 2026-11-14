import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Where the access and refresh tokens are kept.
abstract class TokenStore {
  Future<String?> readAccessToken();
  Future<String?> readRefreshToken();
  Future<void> save({required String accessToken, String? refreshToken});
  Future<void> clear();
}

class SecureTokenStore implements TokenStore {
  const SecureTokenStore();

  static const String _accessTokenKey = 'jwt_token';
  static const String _refreshTokenKey = 'refresh_token';

  static const FlutterSecureStorage _secureStorage = FlutterSecureStorage(
    aOptions: AndroidOptions(),
    iOptions: IOSOptions(
      accessibility: KeychainAccessibility.first_unlock_this_device,
    ),
  );

  @override
  Future<String?> readAccessToken() =>
      _secureStorage.read(key: _accessTokenKey);

  @override
  Future<String?> readRefreshToken() =>
      _secureStorage.read(key: _refreshTokenKey);

  @override
  Future<void> save({required String accessToken, String? refreshToken}) async {
    await _secureStorage.write(key: _accessTokenKey, value: accessToken);

    if (refreshToken != null) {
      await _secureStorage.write(key: _refreshTokenKey, value: refreshToken);
    } else {
      await _secureStorage.delete(key: _refreshTokenKey);
    }
  }

  @override
  Future<void> clear() async {
    await _secureStorage.delete(key: _accessTokenKey);
    await _secureStorage.delete(key: _refreshTokenKey);
  }
}

/// Token store that only lives in memory (tests).
class InMemoryTokenStore implements TokenStore {
  InMemoryTokenStore({this.accessToken, this.refreshToken});

  String? accessToken;
  String? refreshToken;

  @override
  Future<String?> readAccessToken() async => accessToken;

  @override
  Future<String?> readRefreshToken() async => refreshToken;

  @override
  Future<void> save({required String accessToken, String? refreshToken}) async {
    this.accessToken = accessToken;
    this.refreshToken = refreshToken;
  }

  @override
  Future<void> clear() async {
    accessToken = null;
    refreshToken = null;
  }
}

class SessionStorage {
  /// Backing store; replaced in tests.
  static TokenStore store = const SecureTokenStore();

  static Future<String?> getAccessToken() => store.readAccessToken();

  static Future<String?> getRefreshToken() => store.readRefreshToken();

  static Future<void> saveSession({
    required String accessToken,
    String? refreshToken,
  }) => store.save(accessToken: accessToken, refreshToken: refreshToken);

  static Future<void> clear() => store.clear();
}
