import 'package:flutter/material.dart';
import '../core/network/api_client.dart';
import '../core/exceptions/api_exception.dart';
import '../models/friend.dart';

class FriendsController extends ChangeNotifier {
  bool _isLoading = true;
  List<Friend> _friends = [];

  bool get isLoading => _isLoading;
  List<Friend> get friends => _friends;

  List<Friend> get pendingRequests =>
      _friends.where((f) => f.isPending && !f.isSender).toList();

  List<Friend> get acceptedFriends =>
      _friends.where((f) => !f.isPending).toList();

  Future<void> fetchFriends(String? myUserId) async {
    _isLoading = true;
    notifyListeners();

    try {
      final response = await ApiClient.get('/friends');
      final List<dynamic> rawList = List<dynamic>.from(response);

      final List<Friend> fetchedFriends = [];

      for (final item in rawList) {
        final friend = Friend.fromJson(item, myUserId);
        try {
          final userRes = await ApiClient.get('/users/${friend.friendId}');
          friend.username = userRes['username'];
        } catch (_) {
          // Keep null or set to unknown
        }
        fetchedFriends.add(friend);
      }

      _friends = fetchedFriends;
    } catch (e) {
      _friends = [];
      throw 'Failed to fetch friends';
    } finally {
      _isLoading = false;
      notifyListeners();
    }
  }

  Future<void> addFriend(String username, String? myUserId) async {
    try {
      await ApiClient.post('/friends', body: {'username': username});
      await fetchFriends(myUserId);
    } on ApiException catch (e) {
      throw e.message;
    } catch (e) {
      throw 'An error occurred';
    }
  }

  Future<void> handleAction(String endpoint, String? myUserId) async {
    try {
      if (endpoint.endsWith('/accept')) {
        await ApiClient.post(endpoint);
      } else {
        await ApiClient.delete(endpoint);
      }
      await fetchFriends(myUserId);
    } catch (e) {
      throw 'An error occurred';
    }
  }
}
