import 'dart:async';

import 'package:dio/dio.dart';

import '../exceptions/api_exception.dart';
import '../storage/session_storage.dart';

class ApiClient {
  static const String baseUrl = String.fromEnvironment(
    'BACKEND_URL',
    defaultValue: 'http://192.168.1.29:3000',
  );

  static FutureOr<void> Function()? onUnauthorized;

  static final Dio _dio = Dio(
    BaseOptions(
      baseUrl: baseUrl,
      connectTimeout: const Duration(seconds: 5),
      receiveTimeout: const Duration(seconds: 10),
      contentType: Headers.jsonContentType,
    ),
  )..interceptors.add(_AuthInterceptor());

  static Future<String?> getToken() => SessionStorage.getAccessToken();

  static Future<String?> getRefreshToken() => SessionStorage.getRefreshToken();

  static Future<dynamic> _sendRequest(
    Future<Response<dynamic>> Function() request,
  ) async {
    try {
      final response = await request();
      return response.data;
    } on DioException catch (e) {
      throw ApiException(_parseErrorResponse(e), e.response?.statusCode);
    } catch (_) {
      throw ApiException('An unexpected error occurred');
    }
  }

  static String _parseErrorResponse(DioException e) {
    if (e.response?.data != null) {
      final data = e.response!.data;
      if (data is Map<String, dynamic>) {
        if (data.containsKey('details') && data['details'] is List) {
          final details = List<dynamic>.from(data['details']);
          if (details.isNotEmpty && details.length > 1) {
            return details.map((err) => '• $err').join('\n');
          } else if (details.isNotEmpty) {
            return details.first.toString();
          }
        }
        if (data.containsKey('error')) {
          return data['error'].toString();
        }
      } else if (data is String) {
        return data;
      }
    }

    if (e.type == DioExceptionType.connectionError ||
        e.type == DioExceptionType.connectionTimeout ||
        e.type == DioExceptionType.receiveTimeout) {
      return 'Could not connect to the server, try again later';
    }

    if (e.response?.statusCode != null) {
      return 'Error ${e.response!.statusCode}: ${e.response!.statusMessage}';
    }

    return e.message ?? 'An unknown error occurred';
  }

  static Future<dynamic> get(String endpoint) =>
      _sendRequest(() => _dio.get(endpoint));

  static Future<dynamic> post(String endpoint, {Map<String, dynamic>? body}) =>
      _sendRequest(() => _dio.post(endpoint, data: body));

  static Future<dynamic> delete(
    String endpoint, {
    Map<String, dynamic>? body,
  }) => _sendRequest(() => _dio.delete(endpoint, data: body));

  static Future<dynamic> patch(String endpoint, {Map<String, dynamic>? body}) =>
      _sendRequest(() => _dio.patch(endpoint, data: body));
}

class _AuthInterceptor extends QueuedInterceptorsWrapper {
  final Dio _refreshDio = Dio(BaseOptions(baseUrl: ApiClient.baseUrl));

  static const List<String> _excludedPaths = [
    '/auth/login',
    '/auth/register',
    '/auth/refresh',
  ];

  @override
  void onRequest(
    RequestOptions options,
    RequestInterceptorHandler handler,
  ) async {
    final token = await ApiClient.getToken();
    if (token != null) {
      options.headers['Authorization'] = 'Bearer $token';
    }
    return handler.next(options);
  }

  @override
  void onError(DioException err, ErrorInterceptorHandler handler) async {
    if (err.response?.statusCode == 401 &&
        !_shouldSkipRefresh(err.requestOptions.path)) {
      final refreshToken = await ApiClient.getRefreshToken();

      if (refreshToken != null) {
        try {
          final response = await _refreshDio.post(
            '/auth/refresh',
            data: {'refresh_token': refreshToken},
          );

          final newAccessToken = response.data['access_token'];
          final newRefreshToken = response.data['refresh_token'];

          await SessionStorage.saveSession(
            accessToken: newAccessToken,
            refreshToken: newRefreshToken,
          );

          final requestOptions = err.requestOptions;
          requestOptions.headers['Authorization'] = 'Bearer $newAccessToken';

          final retryResponse = await ApiClient._dio.fetch(requestOptions);
          return handler.resolve(retryResponse);
        } catch (_) {
          await ApiClient.onUnauthorized?.call();
          return handler.reject(err);
        }
      } else {
        await ApiClient.onUnauthorized?.call();
        return handler.reject(err);
      }
    }

    return handler.next(err);
  }

  bool _shouldSkipRefresh(String path) {
    return _excludedPaths.any(path.contains);
  }
}
