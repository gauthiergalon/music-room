import 'package:dio/dio.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../exceptions/api_exception.dart';

class ApiClient {
  static const String baseUrl = 'http://192.168.1.29:3000';

  static Function()? onUnauthorized;

  static final Dio _dio = Dio(
    BaseOptions(
      baseUrl: baseUrl,
      connectTimeout: const Duration(seconds: 10),
      receiveTimeout: const Duration(seconds: 10),
      contentType: Headers.jsonContentType,
    ),
  )..interceptors.add(_AuthInterceptor());

  static Future<String?> getToken() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString('jwt_token');
  }

  static Future<String?> getRefreshToken() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString('refresh_token');
  }

  static String _parseErrorResponse(DioException e) {
    if (e.response?.data != null) {
      final data = e.response!.data;
      if (data is Map<String, dynamic>) {
        if (data.containsKey('details') && data['details'] is List) {
          final details = List<String>.from(data['details']);
          if (details.isNotEmpty && details.length > 1) {
            return details.map((err) => '• $err').join('\n');
          } else if (details.isNotEmpty) {
            return details.first;
          }
        }
        if (data.containsKey('error')) {
          return data['error'];
        }
      }
    }

    if (e.type == DioExceptionType.connectionError ||
        e.type == DioExceptionType.connectionTimeout ||
        e.type == DioExceptionType.receiveTimeout) {
      return 'Could not connect to the server, try again later';
    }

    if (e.response?.statusCode != null) {
      return 'Unknown error (${e.response!.statusCode})';
    }

    return e.message ?? 'Network error occurred';
  }

  static Future<dynamic> get(String endpoint) async {
    try {
      final response = await _dio.get(endpoint);
      return response.data;
    } on DioException catch (e) {
      throw ApiException(_parseErrorResponse(e), e.response?.statusCode);
    } catch (e) {
      throw ApiException('An unexpected error occurred');
    }
  }

  static Future<dynamic> post(
    String endpoint, {
    Map<String, dynamic>? body,
  }) async {
    try {
      final response = await _dio.post(endpoint, data: body);
      return response.data;
    } on DioException catch (e) {
      throw ApiException(_parseErrorResponse(e), e.response?.statusCode);
    } catch (e) {
      throw ApiException('An unexpected error occurred');
    }
  }

  static Future<dynamic> patch(
    String endpoint, {
    Map<String, dynamic>? body,
  }) async {
    try {
      final response = await _dio.patch(endpoint, data: body);
      return response.data;
    } on DioException catch (e) {
      throw ApiException(_parseErrorResponse(e), e.response?.statusCode);
    } catch (e) {
      throw ApiException('An unexpected error occurred');
    }
  }
}

class _AuthInterceptor extends QueuedInterceptorsWrapper {
  final Dio _refreshDio = Dio(BaseOptions(baseUrl: ApiClient.baseUrl));

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
        !err.requestOptions.path.contains('/auth/login') &&
        !err.requestOptions.path.contains('/auth/register') &&
        !err.requestOptions.path.contains('/auth/refresh')) {
      final refreshToken = await ApiClient.getRefreshToken();

      if (refreshToken != null) {
        try {
          final response = await _refreshDio.post(
            '/auth/refresh',
            data: {'refresh_token': refreshToken},
          );

          final newAccessToken = response.data['access_token'];
          final newRefreshToken = response.data['refresh_token'];

          final prefs = await SharedPreferences.getInstance();
          await prefs.setString('jwt_token', newAccessToken);
          if (newRefreshToken != null) {
            await prefs.setString('refresh_token', newRefreshToken);
          }

          final requestOptions = err.requestOptions;
          requestOptions.headers['Authorization'] = 'Bearer $newAccessToken';

          final retryResponse = await ApiClient._dio.fetch(requestOptions);
          return handler.resolve(retryResponse);
        } catch (_) {
          ApiClient.onUnauthorized?.call();
        }
      } else {
        ApiClient.onUnauthorized?.call();
      }
    }

    // For all other errors, just forward them
    return handler.next(err);
  }
}
