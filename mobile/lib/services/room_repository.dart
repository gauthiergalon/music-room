import '../core/exceptions/api_exception.dart';
import '../core/network/api_client.dart';
import '../models/room.dart';
import '../models/room_invitation.dart';

class RoomRepository {
  Future<List<Room>> getRooms() async {
    final response = await ApiClient.get('/rooms');
    if (response is! List) return [];
    return response
        .map((data) => Room.fromJson(Map<String, dynamic>.from(data as Map)))
        .toList();
  }

  Future<Room> createRoom() async {
    final response = await ApiClient.post('/rooms');
    return Room.fromJson(Map<String, dynamic>.from(response as Map));
  }

  /// Whether the room still exists and the user may join it. Throws when the
  /// server cannot be reached.
  Future<bool> isRoomAvailable(String roomId) async {
    try {
      await ApiClient.get('/rooms/$roomId');
      return true;
    } on ApiException catch (e) {
      if (e.statusCode == 403 || e.statusCode == 404) return false;
      rethrow;
    }
  }

  Future<void> addTrack(String roomId, int trackId) async {
    await ApiClient.post('/rooms/$roomId/queue', body: {'track_id': trackId});
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

  /// Makes a public room private, or a private room public.
  Future<void> togglePrivacy(String roomId, bool isPublic) async {
    await ApiClient.post(
      isPublic ? '/rooms/$roomId/privatize' : '/rooms/$roomId/publish',
    );
  }

  Future<void> toggleLicense(String roomId, bool isLicensed) async {
    await ApiClient.post(
      isLicensed
          ? '/rooms/$roomId/disable-license'
          : '/rooms/$roomId/enable-license',
    );
  }

  Future<String> getStreamUrl(int trackId) async {
    final response = await ApiClient.get('/hifi/track/$trackId/stream-url');
    final streamUrl = response is Map ? response['stream_url'] : null;

    if (streamUrl is! String || streamUrl.isEmpty) {
      throw ApiException('No stream URL found for track $trackId');
    }

    return streamUrl;
  }

  Future<void> delegateDevice(
    String roomId,
    String userId,
    String connectionId,
  ) async {
    await ApiClient.post(
      '/rooms/$roomId/delegate',
      body: {'user_id': userId, 'connection_id': connectionId},
    );
  }

  Future<void> revokeDelegate(String roomId) async {
    await ApiClient.delete('/rooms/$roomId/delegate');
  }

  Future<void> inviteToRoom(String roomId, String inviteeId) async {
    await ApiClient.post('/rooms/$roomId/invite/$inviteeId');
  }

  Future<List<RoomInvitation>> getPendingInvitations() async {
    final response = await ApiClient.get('/me/invitations');
    if (response is! List) return [];
    return response
        .map(
          (item) =>
              RoomInvitation.fromJson(Map<String, dynamic>.from(item as Map)),
        )
        .where((invitation) => invitation.isPending)
        .toList();
  }

  Future<void> acceptInvitation(String invitationId) async {
    await ApiClient.post('/me/invitations/$invitationId/accept');
  }

  Future<void> rejectInvitation(String invitationId) async {
    await ApiClient.post('/me/invitations/$invitationId/reject');
  }
}
