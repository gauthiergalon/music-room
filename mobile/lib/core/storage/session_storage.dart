import 'package:flutter_secure_storage/flutter_secure_storage.dart';

class SessionStorage {
  static const String accessTokenKey = 'jwt_token';
  static const String refreshTokenKey = 'refresh_token';

  static const FlutterSecureStorage _secureStorage = FlutterSecureStorage(
    aOptions: AndroidOptions(
      encryptedSharedPreferences: true,
    ),
    iOptions: IOSOptions(
      accessibility: KeychainAccessibility.first_unlock_this_device,
    ),
  );

  static Future<String?> getAccessToken() async {
    return _secureStorage.read(key: accessTokenKey);
  }

  static Future<String?> getRefreshToken() async {
    return _secureStorage.read(key: refreshTokenKey);
  }

  static Future<void> saveSession({
    required String accessToken,
    String? refreshToken,
  }) async {
    await _secureStorage.write(key: accessTokenKey, value: accessToken);

    if (refreshToken != null) {
      await _secureStorage.write(key: refreshTokenKey, value: refreshToken);
    } else {
      await _secureStorage.delete(key: refreshTokenKey);
    }
  }

  static Future<void> clear() async {
    await _secureStorage.delete(key: accessTokenKey);
    await _secureStorage.delete(key: refreshTokenKey);
  }
}