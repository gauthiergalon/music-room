import 'package:flutter/foundation.dart';

import '../core/network/api_client.dart';
import '../models/friend.dart';

class FriendsController extends ChangeNotifier {
  bool _isLoading = true;
  List<Friend> _friends = const [];

  bool get isLoading => _isLoading;
  List<Friend> get friends => _friends;

  /// Requests received by the current user that are waiting for an answer.
  List<Friend> get pendingRequests =>
      _friends.where((f) => f.isPending && !f.isSender).toList();

  List<Friend> get acceptedFriends =>
      _friends.where((f) => !f.isPending).toList();

  Future<void> fetchFriends() async {
    _isLoading = true;
    notifyListeners();

    try {
      final response = await ApiClient.get('/friends');
      _friends = [
        if (response is List)
          for (final item in response)
            Friend.fromJson(Map<String, dynamic>.from(item as Map)),
      ];
    } catch (e) {
      _friends = const [];
      rethrow;
    } finally {
      _isLoading = false;
      notifyListeners();
    }
  }

  Future<void> addFriend(String username) async {
    await ApiClient.post('/friends', body: {'username': username});
    await fetchFriends();
  }

  Future<void> acceptRequest(String friendId) async {
    await ApiClient.post('/friends/$friendId/accept');
    await fetchFriends();
  }

  Future<void> rejectRequest(String friendId) async {
    await ApiClient.delete('/friends/$friendId/reject');
    await fetchFriends();
  }

  Future<void> removeFriend(String friendId) async {
    await ApiClient.delete('/friends/$friendId');
    await fetchFriends();
  }

  /// Forgets the friends of the user who just logged out.
  void clear() {
    _friends = const [];
    _isLoading = true;
    notifyListeners();
  }
}
