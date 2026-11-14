import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/models/friend.dart';
import 'package:mobile/models/queue_item.dart';
import 'package:mobile/models/room_invitation.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/models/user.dart';

void main() {
  test('Track.fromJson reads artist, cover and duration', () {
    final track = Track.fromJson({
      'id': 12,
      'title': 'Title',
      'duration': 215,
      'artists': [
        {'name': 'Someone'},
      ],
      'album': {'title': 'Album', 'cover': 'ab-cd-ef'},
    });

    expect(track.id, 12);
    expect(track.artist, 'Someone');
    expect(track.duration, const Duration(seconds: 215));
    expect(
      track.imageUrl,
      'https://resources.tidal.com/images/ab/cd/ef/320x320.jpg',
    );
  });

  test('Track.fromJson falls back when artist and album are missing', () {
    final track = Track.fromJson({'id': 1, 'title': 'T', 'duration': 0});
    expect(track.artist, 'Unknown Artist');
    expect(track.imageUrl, isNull);
  });

  test('QueueItem.fromJson carries the track of the payload', () {
    final item = QueueItem.fromJson({
      'id': 'q1',
      'position': 3,
      'track': {'id': 5, 'title': 'Queued', 'duration': 60},
    });
    expect(item.id, 'q1');
    expect(item.position, 3.0);
    expect(item.track.title, 'Queued');
    expect(item.trackId, 5);
  });

  test('Friend.fromJson tells who sent the request', () {
    final received = Friend.fromJson({
      'friend_id': 'other',
      'username': 'bob',
      'sender_id': 'other',
      'is_pending': true,
    });
    expect(received.isSender, isFalse);
    expect(received.username, 'bob');

    final sent = Friend.fromJson({
      'friend_id': 'other',
      'username': 'bob',
      'sender_id': 'me',
      'is_pending': true,
    });
    expect(sent.isSender, isTrue);
  });

  test('RoomInvitation.fromJson reads the inviter and room names', () {
    final invitation = RoomInvitation.fromJson({
      'id': 'i1',
      'room_id': 'r1',
      'inviter_id': 'u1',
      'is_pending': true,
      'inviter_username': 'alice',
      'room_name': "alice's room",
    });
    expect(invitation.inviterUsername, 'alice');
    expect(invitation.roomName, "alice's room");
  });

  test('User.fromJson reads the account flags', () {
    final user = User.fromJson({
      'id': 'u1',
      'username': 'alice',
      'email': 'a@example.com',
      'email_confirmed': true,
      'google_linked': true,
      'has_password': false,
      'favorite_genres': ['jazz', ''],
      'privacy_level': 'Public',
      'is_subscribed': true,
      'end_subscription_date': '2026-12-01T00:00:00Z',
    });
    expect(user.googleLinked, isTrue);
    expect(user.hasPassword, isFalse);
    expect(user.favoriteGenres, ['jazz']);
    expect(user.endSubscriptionDate, DateTime.utc(2026, 12));
  });

  test('PublicProfile hides genres the server did not send', () {
    final hidden = PublicProfile.fromJson({
      'id': 'u1',
      'username': 'alice',
      'favorite_genres': null,
      'privacy_level': 'Private',
    });
    expect(hidden.favoriteGenres, isNull);

    final visible = PublicProfile.fromJson({
      'id': 'u1',
      'username': 'alice',
      'favorite_genres': <String>[],
      'privacy_level': 'Public',
    });
    expect(visible.favoriteGenres, isEmpty);
  });
}
