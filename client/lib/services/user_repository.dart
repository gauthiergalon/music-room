import '../core/network/api_client.dart';
import '../models/user.dart';

class UserRepository {
  const UserRepository();

  Future<PublicProfile> getPublicProfile(
    String userId, {
    String? fallbackUsername,
  }) async {
    final data = await ApiClient.get('/users/$userId');
    return PublicProfile.fromJson(
      Map<String, dynamic>.from(data as Map),
      fallbackUsername: fallbackUsername,
    );
  }
}
