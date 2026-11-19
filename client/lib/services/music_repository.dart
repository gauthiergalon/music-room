import '../core/network/api_client.dart';
import '../models/track.dart';

class MusicRepository {
  const MusicRepository();

  Future<List<Track>> searchTracks(String query) async {
    final response = await ApiClient.get(
      '/hifi/search/${Uri.encodeComponent(query)}',
    );
    final data = response is Map ? response['data'] : null;
    final items = data is Map ? data['items'] : null;
    if (items is! List) return [];
    return items
        .map((json) => Track.fromJson(Map<String, dynamic>.from(json as Map)))
        .toList();
  }
}
