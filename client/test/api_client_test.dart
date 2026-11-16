import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/core/exceptions/api_exception.dart';
import 'package:mobile/core/network/api_client.dart';
import 'package:mobile/core/storage/session_storage.dart';

typedef _Handler = FutureOr<ResponseBody> Function(_Request request);

/// What the fake server saw for one request.
class _Request {
  _Request(RequestOptions options)
    : method = options.method,
      path = options.path,
      authorization = options.headers['Authorization']?.toString(),
      device = options.headers['X-Device']?.toString();

  final String method;
  final String path;
  final String? authorization;
  final String? device;
}

class _FakeServer implements HttpClientAdapter {
  _FakeServer(this._handler);

  final _Handler _handler;
  final List<_Request> requests = [];

  int count(String path) => requests.where((r) => r.path == path).length;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    final request = _Request(options);
    requests.add(request);
    return _handler(request);
  }

  @override
  void close({bool force = false}) {}
}

ResponseBody _json(int status, Object? body) => ResponseBody.fromString(
  jsonEncode(body),
  status,
  headers: {
    Headers.contentTypeHeader: [Headers.jsonContentType],
  },
);

ResponseBody _error(int status, String message) => _json(status, {
  'error': 'Error',
  'details': [message],
});

String _jwt(DateTime expiry) {
  String part(Object value) =>
      base64Url.encode(utf8.encode(jsonEncode(value))).replaceAll('=', '');
  final payload = {'exp': expiry.millisecondsSinceEpoch ~/ 1000};
  return '${part({'alg': 'HS256'})}.${part(payload)}.signature';
}

const _timeout = Duration(seconds: 2);

void main() {
  late InMemoryTokenStore store;
  late int unauthorizedCalls;

  ApiHttp buildHttp(_FakeServer server) => ApiHttp(
    baseUrl: 'http://backend.test',
    tokenStore: store,
    deviceHeaders: const {'X-Device': 'Test device'},
    adapter: server,
    onUnauthorized: () => unauthorizedCalls++,
  );

  /// A server where only [validToken] is accepted and `/auth/refresh` hands
  /// out `new-access` in exchange for `refresh-1`.
  _FakeServer refreshingServer({
    FutureOr<ResponseBody> Function(_Request request)? onAuthorized,
    Duration refreshDelay = Duration.zero,
  }) {
    return _FakeServer((request) async {
      if (request.path == '/auth/refresh') {
        if (refreshDelay > Duration.zero) {
          await Future<void>.delayed(refreshDelay);
        }
        return _json(200, {
          'access_token': 'new-access',
          'refresh_token': 'refresh-2',
        });
      }
      if (request.authorization != 'Bearer new-access') {
        return _error(401, 'Invalid token');
      }
      return onAuthorized?.call(request) ?? _json(200, {'ok': true});
    });
  }

  setUp(() {
    store = InMemoryTokenStore(
      accessToken: 'old-access',
      refreshToken: 'refresh-1',
    );
    unauthorizedCalls = 0;
  });

  test('sends the access token and the device headers', () async {
    final server = _FakeServer((_) => _json(200, {'ok': true}));
    final http = buildHttp(server);

    final data = await http.get('/users/me');

    expect(data, {'ok': true});
    expect(server.requests.single.authorization, 'Bearer old-access');
    expect(server.requests.single.device, 'Test device');
  });

  test('refreshes an expired token and replays the request', () async {
    final server = refreshingServer();
    final http = buildHttp(server);

    final data = await http.get('/users/me').timeout(_timeout);

    expect(data, {'ok': true});
    expect(server.count('/auth/refresh'), 1);
    expect(server.requests.last.authorization, 'Bearer new-access');
    expect(server.requests.last.device, 'Test device');
    expect(store.accessToken, 'new-access');
    expect(store.refreshToken, 'refresh-2');
    expect(unauthorizedCalls, 0);
  });

  test('a replayed request that fails completes with its error', () async {
    final server = refreshingServer(onAuthorized: (_) => _error(500, 'Boom'));
    final http = buildHttp(server);

    await expectLater(
      http.get('/rooms').timeout(_timeout),
      throwsA(
        isA<ApiException>()
            .having((e) => e.statusCode, 'statusCode', 500)
            .having((e) => e.message, 'message', 'Boom'),
      ),
    );

    // The client is still usable: a later failing request completes too.
    await expectLater(
      http.get('/rooms/unknown').timeout(_timeout),
      throwsA(isA<ApiException>().having((e) => e.statusCode, 'status', 500)),
    );
    expect(unauthorizedCalls, 0);
  });

  test('concurrent expired requests share a single refresh', () async {
    final server = refreshingServer(
      refreshDelay: const Duration(milliseconds: 20),
    );
    final http = buildHttp(server);

    final results = await Future.wait([
      http.get('/a'),
      http.get('/b'),
      http.get('/c'),
    ]).timeout(_timeout);

    expect(results, everyElement({'ok': true}));
    expect(server.count('/auth/refresh'), 1);
  });

  test(
    'a request that used an already replaced token is just replayed',
    () async {
      final server = _FakeServer((request) {
        if (request.path == '/auth/refresh') return _error(500, 'unexpected');
        if (request.authorization == 'Bearer new-access') {
          return _json(200, {'ok': true});
        }
        // Another caller refreshed the session while this request was in
        // flight: the stored token is already the new one.
        store
          ..accessToken = 'new-access'
          ..refreshToken = 'refresh-2';
        return _error(401, 'Invalid token');
      });
      final http = buildHttp(server);

      final data = await http.get('/a').timeout(_timeout);

      expect(data, {'ok': true});
      expect(server.count('/auth/refresh'), 0);
      expect(server.count('/a'), 2);
    },
  );

  test('a rejected refresh ends the session', () async {
    final server = _FakeServer((_) => _error(401, 'Invalid token'));
    final http = buildHttp(server);

    await expectLater(
      http.get('/users/me').timeout(_timeout),
      throwsA(isA<ApiException>().having((e) => e.statusCode, 'status', 401)),
    );
    expect(server.count('/auth/refresh'), 1);
    expect(unauthorizedCalls, 1);
  });

  test('a refresh that cannot reach the server keeps the session', () async {
    final server = _FakeServer((request) {
      if (request.path == '/auth/refresh') {
        throw DioException.connectionError(
          requestOptions: RequestOptions(path: request.path),
          reason: 'offline',
        );
      }
      return _error(401, 'Invalid token');
    });
    final http = buildHttp(server);

    await expectLater(
      http.get('/users/me').timeout(_timeout),
      throwsA(isA<ApiException>().having((e) => e.statusCode, 'status', 401)),
    );
    expect(unauthorizedCalls, 0);
    expect(store.accessToken, 'old-access');
  });

  test('wrong credentials do not trigger a refresh', () async {
    final server = _FakeServer((_) => _error(401, 'Invalid email or password'));
    final http = buildHttp(server);

    await expectLater(
      http.post('/auth/login', body: {'email': 'a@b.c', 'password': 'x'}),
      throwsA(
        isA<ApiException>().having(
          (e) => e.message,
          'message',
          'Invalid email or password',
        ),
      ),
    );
    expect(server.count('/auth/refresh'), 0);
    expect(unauthorizedCalls, 0);
  });

  test('several error details are listed', () async {
    final server = _FakeServer(
      (_) => _json(422, {
        'error': 'Validation Error',
        'details': ['First', 'Second'],
      }),
    );
    final http = buildHttp(server);

    await expectLater(
      http.post('/auth/register'),
      throwsA(
        isA<ApiException>().having(
          (e) => e.message,
          'message',
          '• First\n• Second',
        ),
      ),
    );
  });

  group('ensureFreshToken', () {
    test('returns null without a session', () async {
      store = InMemoryTokenStore();
      final server = _FakeServer((_) => _json(200, {}));

      expect(await buildHttp(server).ensureFreshToken(), isNull);
      expect(server.requests, isEmpty);
    });

    test('keeps a token that is still valid for a while', () async {
      final token = _jwt(DateTime.now().add(const Duration(minutes: 10)));
      store.accessToken = token;
      final server = _FakeServer((_) => _json(200, {}));

      expect(await buildHttp(server).ensureFreshToken(), token);
      expect(server.requests, isEmpty);
    });

    test('refreshes a token that is about to expire', () async {
      store.accessToken = _jwt(DateTime.now().add(const Duration(seconds: 10)));
      final server = refreshingServer();

      expect(await buildHttp(server).ensureFreshToken(), 'new-access');
      expect(server.count('/auth/refresh'), 1);
      expect(store.accessToken, 'new-access');
    });

    test('ends the session when the refresh is rejected', () async {
      store.accessToken = _jwt(
        DateTime.now().subtract(const Duration(minutes: 1)),
      );
      final server = _FakeServer((_) => _error(401, 'Invalid token'));

      expect(await buildHttp(server).ensureFreshToken(), isNull);
      expect(unauthorizedCalls, 1);
    });
  });

  group('jwtExpiry', () {
    test('reads the exp claim', () {
      final expiry = DateTime.utc(2030, 1, 2, 3, 4, 5);
      expect(jwtExpiry(_jwt(expiry)), expiry);
    });

    test('returns null for something that is not a JWT', () {
      expect(jwtExpiry('not-a-token'), isNull);
      expect(jwtExpiry('a.b.c'), isNull);
    });
  });
}
