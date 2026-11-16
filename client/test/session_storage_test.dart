import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/core/storage/session_storage.dart';

/// A store that counts how often it is actually read.
class _CountingStore extends InMemoryTokenStore {
  _CountingStore({super.accessToken, super.refreshToken});

  int reads = 0;

  @override
  Future<String?> readAccessToken() {
    reads++;
    return super.readAccessToken();
  }

  @override
  Future<String?> readRefreshToken() {
    reads++;
    return super.readRefreshToken();
  }
}

void main() {
  group('CachedTokenStore', () {
    test(
      'reads the underlying storage once, however many requests ask',
      () async {
        final storage = _CountingStore(accessToken: 'a1', refreshToken: 'r1');
        final store = CachedTokenStore(storage);

        // Concurrent first readers share one load.
        final tokens = await Future.wait([
          store.readAccessToken(),
          store.readAccessToken(),
          store.readRefreshToken(),
        ]);
        for (var i = 0; i < 20; i++) {
          await store.readAccessToken();
        }

        expect(tokens, ['a1', 'a1', 'r1']);
        expect(
          storage.reads,
          2,
        ); // one access token read, one refresh token read
      },
    );

    test('a saved session is served from memory and written through', () async {
      final storage = _CountingStore();
      final store = CachedTokenStore(storage);

      await store.save(accessToken: 'a2', refreshToken: 'r2');

      expect(await store.readAccessToken(), 'a2');
      expect(await store.readRefreshToken(), 'r2');
      expect(storage.accessToken, 'a2');
      expect(storage.refreshToken, 'r2');
      expect(storage.reads, 0);
    });

    test('clearing forgets the tokens in memory and in the storage', () async {
      final storage = _CountingStore(accessToken: 'a1', refreshToken: 'r1');
      final store = CachedTokenStore(storage);
      await store.readAccessToken();

      await store.clear();

      expect(await store.readAccessToken(), isNull);
      expect(await store.readRefreshToken(), isNull);
      expect(storage.accessToken, isNull);
    });

    test('invalidate picks up a session renewed by another tab', () async {
      final storage = _CountingStore(accessToken: 'a1', refreshToken: 'r1');
      final store = CachedTokenStore(storage);
      expect(await store.readAccessToken(), 'a1');

      // Another instance of the app wrote new tokens to the shared storage.
      storage.accessToken = 'a2';
      storage.refreshToken = 'r2';
      expect(await store.readAccessToken(), 'a1', reason: 'still cached');

      store.invalidate();

      expect(await store.readAccessToken(), 'a2');
      expect(await store.readRefreshToken(), 'r2');
    });
  });
}
