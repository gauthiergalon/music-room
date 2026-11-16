import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Where the access and refresh tokens are kept.
abstract class TokenStore {
  Future<String?> readAccessToken();
  Future<String?> readRefreshToken();
  Future<void> save({required String accessToken, String? refreshToken});
  Future<void> clear();

  /// Forgets any copy kept in memory, so the next read comes from the
  /// underlying storage. Called before a refresh: another instance of the
  /// app (a second browser tab) may have renewed the session meanwhile.
  void invalidate();
}

/// Keeps the tokens of [inner] in memory after the first read.
///
/// Every request needs the access token, and reading the secure storage is a
/// round trip to the platform (or a decryption in the browser) each time.
class CachedTokenStore implements TokenStore {
  CachedTokenStore(this.inner);

  final TokenStore inner;

  bool _loaded = false;
  String? _accessToken;
  String? _refreshToken;

  /// Shared by concurrent readers so the storage is read once.
  Future<void>? _loading;

  Future<void> _ensureLoaded() {
    if (_loaded) return Future.value();
    return _loading ??= _load().whenComplete(() => _loading = null);
  }

  Future<void> _load() async {
    final accessToken = await inner.readAccessToken();
    final refreshToken = await inner.readRefreshToken();
    // A save, clear or invalidate that happened meanwhile wins.
    if (_loaded) return;
    _accessToken = accessToken;
    _refreshToken = refreshToken;
    _loaded = true;
  }

  @override
  Future<String?> readAccessToken() async {
    await _ensureLoaded();
    return _accessToken;
  }

  @override
  Future<String?> readRefreshToken() async {
    await _ensureLoaded();
    return _refreshToken;
  }

  @override
  Future<void> save({required String accessToken, String? refreshToken}) async {
    _accessToken = accessToken;
    _refreshToken = refreshToken;
    _loaded = true;
    await inner.save(accessToken: accessToken, refreshToken: refreshToken);
  }

  @override
  Future<void> clear() async {
    _accessToken = null;
    _refreshToken = null;
    _loaded = true;
    await inner.clear();
  }

  @override
  void invalidate() {
    _loaded = false;
    inner.invalidate();
  }
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

  @override
  void invalidate() {}
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

  @override
  void invalidate() {}
}

class SessionStorage {
  /// Backing store; replaced in tests.
  static TokenStore store = CachedTokenStore(const SecureTokenStore());

  static Future<String?> getAccessToken() => store.readAccessToken();

  static Future<String?> getRefreshToken() => store.readRefreshToken();

  static Future<void> saveSession({
    required String accessToken,
    String? refreshToken,
  }) => store.save(accessToken: accessToken, refreshToken: refreshToken);

  static Future<void> clear() => store.clear();
}
