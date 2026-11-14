import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/services/room_repository.dart';

void main() {
  group('RoomRepository.resolveStreamUrl', () {
    test('keeps the provider URL native apps receive', () {
      const url = 'https://im-fa.manifest.tidal.com/1/manifests/abc.m3u8?x=1';

      expect(
        RoomRepository.resolveStreamUrl(url, 'http://192.168.1.11:3000'),
        url,
      );
    });

    test('resolves a relayed path against the API', () {
      expect(
        RoomRepository.resolveStreamUrl(
          '/hifi/stream/playlist.m3u8?u=abc&exp=1&sig=ff',
          'http://192.168.1.11:3000',
        ),
        'http://192.168.1.11:3000/hifi/stream/playlist.m3u8?u=abc&exp=1&sig=ff',
      );
    });

    test('keeps the path prefix of an API served under a sub-path', () {
      expect(
        RoomRepository.resolveStreamUrl(
          '/hifi/stream/playlist.m3u8?u=abc',
          'https://example.com/api/',
        ),
        'https://example.com/api/hifi/stream/playlist.m3u8?u=abc',
      );
    });
  });
}
