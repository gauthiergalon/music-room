import 'package:flutter/foundation.dart';

import '../models/room_invitation.dart';
import '../services/room_repository.dart';

/// Room invitations received by the current user.
class InvitationsController extends ChangeNotifier {
  InvitationsController({RoomRepository? roomRepository})
    : _roomRepository = roomRepository ?? RoomRepository();

  final RoomRepository _roomRepository;

  bool _isLoading = false;
  List<RoomInvitation> _pending = const [];
  final Set<String> _processingIds = {};

  bool get isLoading => _isLoading;
  List<RoomInvitation> get pending => _pending;

  bool isProcessing(String invitationId) =>
      _processingIds.contains(invitationId);

  Future<void> fetch() async {
    _isLoading = true;
    notifyListeners();

    try {
      _pending = await _roomRepository.getPendingInvitations();
    } finally {
      _isLoading = false;
      notifyListeners();
    }
  }

  Future<void> accept(RoomInvitation invitation) =>
      _answer(invitation, _roomRepository.acceptInvitation);

  Future<void> reject(RoomInvitation invitation) =>
      _answer(invitation, _roomRepository.rejectInvitation);

  Future<void> _answer(
    RoomInvitation invitation,
    Future<void> Function(String id) send,
  ) async {
    if (!_processingIds.add(invitation.id)) return;
    notifyListeners();

    try {
      await send(invitation.id);
      _pending = _pending.where((i) => i.id != invitation.id).toList();
    } finally {
      _processingIds.remove(invitation.id);
      notifyListeners();
    }
  }

  /// Forgets the invitations of the user who just logged out.
  void clear() {
    _pending = const [];
    _processingIds.clear();
    _isLoading = false;
    notifyListeners();
  }
}
