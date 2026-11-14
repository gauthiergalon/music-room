import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_dotenv/flutter_dotenv.dart';
import 'package:package_info_plus/package_info_plus.dart';

import '../exceptions/api_exception.dart';
import '../logger/logger.dart';
import '../storage/session_storage.dart';

/// The refresh token was rejected: the user has to log in again.
class SessionExpiredException implements Exception {
  const SessionExpiredException();

  @override
  String toString() => 'Session expired';
}

/// Expiry of a JWT access token, or `null` when it cannot be read.
DateTime? jwtExpiry(String token) {
  final parts = token.split('.');
  if (parts.length != 3) return null;
  try {
    final payload = jsonDecode(
      utf8.decode(base64Url.decode(base64Url.normalize(parts[1]))),
    );
    final exp = payload is Map ? payload['exp'] : null;
    if (exp is! num) return null;
    return DateTime.fromMillisecondsSinceEpoch(exp.toInt() * 1000, isUtc: true);
  } catch (_) {
    return null;
  }
}

/// HTTP access to the backend: attaches the access token, refreshes it once
/// for all concurrent callers when it expires, and retries the failed request.
class ApiHttp {
  ApiHttp({
    required this.baseUrl,
    required this.tokenStore,
    Map<String, String> deviceHeaders = const {},
    HttpClientAdapter? adapter,
    this.onUnauthorized,
  }) {
    BaseOptions options() => BaseOptions(
      baseUrl: baseUrl,
      connectTimeout: const Duration(seconds: 5),
      receiveTimeout: const Duration(seconds: 10),
      contentType: Headers.jsonContentType,
      headers: deviceHeaders,
    );

    dio = Dio(options())..interceptors.add(_AuthInterceptor(this));
    // No auth interceptor here: this client sends the refresh request and
    // replays requests after a refresh, so their failures go straight back to
    // the caller instead of re-entering the interceptor.
    _plain = Dio(options());

    if (adapter != null) {
      dio.httpClientAdapter = adapter;
      _plain.httpClientAdapter = adapter;
    }
  }

  final String baseUrl;
  final TokenStore tokenStore;

  /// Called when the session cannot be refreshed any more.
  void Function()? onUnauthorized;

  late final Dio dio;
  late final Dio _plain;

  Future<String>? _refreshInFlight;

  /// Returns a valid access token, refreshing the session if needed.
  ///
  /// [staleToken] is the token the caller found to be expired. If the stored
  /// token is already a different one, another caller refreshed in the
  /// meantime and that token is returned as is. Concurrent calls share one
  /// refresh request.
  Future<String> refreshAccessToken({String? staleToken}) {
    final inFlight = _refreshInFlight;
    if (inFlight != null) return inFlight;

    final future = _refresh(staleToken);
    _refreshInFlight = future;
    return future.whenComplete(() {
      if (identical(_refreshInFlight, future)) _refreshInFlight = null;
    });
  }

  Future<String> _refresh(String? staleToken) async {
    final current = await tokenStore.readAccessToken();
    if (staleToken != null && current != null && current != staleToken) {
      return current;
    }

    final refreshToken = await tokenStore.readRefreshToken();
    if (refreshToken == null) throw const SessionExpiredException();

    try {
      final response = await _plain.post(
        '/auth/refresh',
        data: {'refresh_token': refreshToken},
      );
      final data = response.data;
      final accessToken = data is Map ? data['access_token'] : null;
      if (accessToken is! String || accessToken.isEmpty) {
        throw const SessionExpiredException();
      }
      await tokenStore.save(
        accessToken: accessToken,
        refreshToken: data['refresh_token']?.toString(),
      );
      logger.debug('Token refreshed successfully');
      return accessToken;
    } on DioException catch (e) {
      final status = e.response?.statusCode;
      if (status == 401 || status == 403) {
        throw const SessionExpiredException();
      }
      rethrow;
    }
  }

  /// Access token that is valid for at least [minValidity], or `null` when
  /// there is no usable session.
  ///
  /// Used before opening a WebSocket, which cannot go through the interceptor.
  Future<String?> ensureFreshToken({
    Duration minValidity = const Duration(seconds: 60),
  }) async {
    final token = await tokenStore.readAccessToken();
    if (token == null) return null;

    final expiry = jwtExpiry(token);
    if (expiry == null ||
        expiry.difference(DateTime.now().toUtc()) > minValidity) {
      return token;
    }

    try {
      return await refreshAccessToken(staleToken: token);
    } on SessionExpiredException {
      onUnauthorized?.call();
      return null;
    } catch (e) {
      // Network problem: the current token may still work for a moment.
      logger.warning('Could not refresh token before it expires');
      return token;
    }
  }

  Future<dynamic> get(String endpoint, {Map<String, dynamic>? query}) =>
      _send(() => dio.get(endpoint, queryParameters: query));

  Future<dynamic> post(String endpoint, {Map<String, dynamic>? body}) =>
      _send(() => dio.post(endpoint, data: body));

  Future<dynamic> delete(String endpoint, {Map<String, dynamic>? body}) =>
      _send(() => dio.delete(endpoint, data: body));

  Future<dynamic> patch(String endpoint, {Map<String, dynamic>? body}) =>
      _send(() => dio.patch(endpoint, data: body));

  Future<dynamic> _send(Future<Response<dynamic>> Function() request) async {
    try {
      final response = await request();
      return response.data;
    } on DioException catch (e, st) {
      logger.error(
        'API Error: ${e.requestOptions.method} ${e.requestOptions.path}',
        error: e,
        stackTrace: st,
      );
      throw ApiException(_parseErrorResponse(e), e.response?.statusCode);
    } catch (e, st) {
      logger.error('Unexpected error', error: e, stackTrace: st);
      throw ApiException('An unexpected error occurred');
    }
  }

  static String _parseErrorResponse(DioException e) {
    final data = e.response?.data;
    if (data is Map) {
      final details = data['details'];
      if (details is List && details.isNotEmpty) {
        return details.length > 1
            ? details.map((err) => '• $err').join('\n')
            : details.first.toString();
      }
      if (data['error'] != null) return data['error'].toString();
    } else if (data is String && data.isNotEmpty) {
      return data;
    }

    if (e.type == DioExceptionType.connectionError ||
        e.type == DioExceptionType.connectionTimeout ||
        e.type == DioExceptionType.receiveTimeout) {
      return 'Could not connect to the server, try again later';
    }

    final status = e.response?.statusCode;
    if (status != null) {
      return 'Error $status: ${e.response!.statusMessage}';
    }

    return e.message ?? 'An unknown error occurred';
  }
}

class _AuthInterceptor extends Interceptor {
  _AuthInterceptor(this._http);

  final ApiHttp _http;

  static const String _retriedKey = 'auth_retried';

  /// Endpoints where a 401 is an answer, not an expired session.
  static const List<String> _noRefreshPaths = [
    '/auth/login',
    '/auth/register',
    '/auth/refresh',
    '/auth/logout',
    '/auth/google-login',
    '/auth/forgot-password',
    '/auth/reset-password',
  ];

  @override
  Future<void> onRequest(
    RequestOptions options,
    RequestInterceptorHandler handler,
  ) async {
    logger.debug('${options.method} ${options.path}');
    final token = await _http.tokenStore.readAccessToken();
    if (token != null) {
      options.headers['Authorization'] = 'Bearer $token';
    }
    handler.next(options);
  }

  @override
  Future<void> onError(
    DioException err,
    ErrorInterceptorHandler handler,
  ) async {
    final request = err.requestOptions;
    final expired =
        err.response?.statusCode == 401 &&
        request.extra[_retriedKey] != true &&
        !_noRefreshPaths.any(request.path.contains);

    if (!expired) return handler.next(err);

    final String token;
    try {
      token = await _http.refreshAccessToken(staleToken: _bearerOf(request));
    } on SessionExpiredException {
      logger.warning('Session expired');
      _http.onUnauthorized?.call();
      return handler.next(err);
    } catch (e) {
      // The refresh could not reach the server. Keep the session: the user
      // is offline, not logged out.
      logger.error('Token refresh failed', error: e);
      return handler.next(err);
    }

    request.headers['Authorization'] = 'Bearer $token';
    request.extra[_retriedKey] = true;

    try {
      handler.resolve(await _http._plain.fetch<dynamic>(request));
    } on DioException catch (retryError) {
      handler.next(retryError);
    }
  }

  static String? _bearerOf(RequestOptions request) {
    final header = request.headers['Authorization'];
    if (header is! String || !header.startsWith('Bearer ')) return null;
    return header.substring('Bearer '.length);
  }
}

/// App-wide entry point to the backend.
class ApiClient {
  static ApiHttp? _http;
  static String _deviceName = 'Unknown device';
  static void Function()? _onUnauthorized;

  static ApiHttp get _client {
    final http = _http;
    if (http == null) {
      throw StateError('ApiClient.init() must be called before any request');
    }
    return http;
  }

  static String get deviceName => _deviceName;

  static String get baseUrl => _client.baseUrl;

  /// Synchronous callback triggered when the session cannot be refreshed.
  /// Implementations must clear state synchronously (start any async
  /// cleanup in the background).
  static void Function()? get onUnauthorized => _onUnauthorized;

  static set onUnauthorized(void Function()? callback) {
    _onUnauthorized = callback;
    _http?.onUnauthorized = callback;
  }

  static Future<void> init() async {
    final backendUrl = dotenv.env['BACKEND_URL']?.trim();
    if (backendUrl == null || backendUrl.isEmpty) {
      throw StateError('BACKEND_URL must be provided in .env');
    }

    final deviceHeaders = await _deviceHeaders();
    _deviceName = deviceHeaders['X-Device'] ?? 'Unknown device';
    _http = ApiHttp(
      baseUrl: backendUrl,
      tokenStore: SessionStorage.store,
      deviceHeaders: deviceHeaders,
      onUnauthorized: _onUnauthorized,
    );
  }

  /// Replaces the HTTP layer (tests).
  @visibleForTesting
  static void useForTesting(ApiHttp http) {
    _http = http..onUnauthorized = _onUnauthorized;
  }

  static Future<Map<String, String>> _deviceHeaders() async {
    final packageInfo = await PackageInfo.fromPlatform();

    final Map<String, String> headers;
    if (kIsWeb) {
      headers = {'X-Platform': 'web', 'X-Device': 'Web'};
    } else if (Platform.isAndroid) {
      final info = await DeviceInfoPlugin().androidInfo;
      headers = {'X-Platform': 'android', 'X-Device': info.model};
    } else if (Platform.isIOS) {
      final info = await DeviceInfoPlugin().iosInfo;
      headers = {'X-Platform': 'ios', 'X-Device': info.utsname.machine};
    } else {
      headers = {'X-Platform': Platform.operatingSystem};
    }

    return {...headers, 'X-App-Version': packageInfo.version};
  }

  static Future<String?> getToken() => SessionStorage.getAccessToken();

  static Future<String?> getRefreshToken() => SessionStorage.getRefreshToken();

  static Future<String?> ensureFreshToken() => _client.ensureFreshToken();

  static Future<dynamic> get(String endpoint, {Map<String, dynamic>? query}) =>
      _client.get(endpoint, query: query);

  static Future<dynamic> post(String endpoint, {Map<String, dynamic>? body}) =>
      _client.post(endpoint, body: body);

  static Future<dynamic> delete(
    String endpoint, {
    Map<String, dynamic>? body,
  }) => _client.delete(endpoint, body: body);

  static Future<dynamic> patch(String endpoint, {Map<String, dynamic>? body}) =>
      _client.patch(endpoint, body: body);
}
