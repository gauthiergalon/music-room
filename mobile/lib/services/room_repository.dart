import '../core/network/api_client.dart';
import '../models/room.dart';

class RoomRepository {
  Future<List<Room>> getRooms() async {
    final response = await ApiClient.get('/rooms');
    if (response is List) {
      return response.map((data) => Room.fromJson(data)).toList();
    }
    return [];
  }

  Future<Room> createRoom() async {
    final response = await ApiClient.post('/rooms');
    return Room.fromJson(response);
  }

  Future<void> addTrack(String roomId, int trackId) async {
    await ApiClient.post(
      '/rooms/$roomId/queue',
      body: {'track_id': trackId},
    );
  }

  Future<void> removeQueueItem(String roomId, String queueItemId) async {
    await ApiClient.delete('/rooms/$roomId/queue', body: {'id': queueItemId});
  }

  Future<void> reorderQueueItem(
    String roomId,
    String queueItemId,
    double newPosition,
  ) async {
    await ApiClient.patch(
      '/rooms/$roomId/queue',
      body: {'id': queueItemId, 'new_position': newPosition},
    );
  }

  Future<void> togglePrivacy(String roomId, bool isPublic) async {
    if (isPublic) {
      await ApiClient.post('/rooms/$roomId/privatize');
    } else {
      await ApiClient.post('/rooms/$roomId/publish');
    }
  }

  Future<void> toggleLicense(String roomId, bool isLicensed) async {
    if (isLicensed) {
      await ApiClient.post('/rooms/$roomId/disable-license');
    } else {
      await ApiClient.post('/rooms/$roomId/enable-license');
    }
  }

  Future<void> transferOwnership(String roomId, String newOwnerId) async {
    await ApiClient.post(
      '/rooms/$roomId/transfer-ownership',
      body: {'new_owner_id': newOwnerId},
    );
  }

  Future<String> getStreamUrl(int trackId) async {
    final response = await ApiClient.get('/hifi/track/$trackId/stream-url');
    final streamUrl = response['stream_url'] as String?;

    if (streamUrl == null || streamUrl.isEmpty) {
      throw Exception('No stream URL found for track $trackId');
    }

    return streamUrl;
  }
}
