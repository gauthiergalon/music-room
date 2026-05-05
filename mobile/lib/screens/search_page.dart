import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/controllers/room_controller.dart';
import '../core/theme.dart';
import '../core/network/api_client.dart';
import '../core/utils/ui_utils.dart';
import '../widgets/track_list_tile.dart';

class SearchPage extends StatefulWidget {
  const SearchPage({super.key});

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final TextEditingController _searchController = TextEditingController();
  bool _isLoading = false;
  bool _hasSearched = false;
  List<Track> _results = [];

  void _performSearch(String query) async {
    if (query.trim().isEmpty) {
      setState(() {
        _results = [];
        _hasSearched = false;
      });
      return;
    }

    setState(() {
      _isLoading = true;
      _hasSearched = true;
    });

    try {
      final response = await ApiClient.get('/hifi/search/$query');

      if (!mounted) return;

      if (response != null &&
          response['data'] != null &&
          response['data']['items'] != null) {
        final List<dynamic> itemsJson = response['data']['items'];
        setState(() {
          _results = itemsJson.map((json) => Track.fromJson(json)).toList();
          _isLoading = false;
        });
      } else {
        setState(() {
          _results = [];
          _isLoading = false;
        });
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _isLoading = false;
      });
      UiUtils.showError(context, 'Error: $e');
    }
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        bottom: false,
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.only(
                top: AppTheme.spacingMd,
                left: AppTheme.spacingMd,
                right: AppTheme.spacingMd,
              ),
              child: Material(
                borderRadius: BorderRadius.circular(12),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 4.0),
                  child: SearchBar(
                    controller: _searchController,
                    hintText: 'Find a song...',
                    leading: const Padding(
                      padding: EdgeInsets.only(left: 8.0),
                      child: Icon(Icons.search),
                    ),
                    trailing: [
                      if (_searchController.text.isNotEmpty)
                        IconButton(
                          icon: const Icon(Icons.clear),
                          onPressed: () {
                            _searchController.clear();
                            _performSearch('');
                          },
                        ),
                    ],
                    onSubmitted: _performSearch,
                    onChanged: (value) {
                      setState(() {});
                    },
                  ),
                ),
              ),
            ),
            const SizedBox(height: 8),
            Expanded(child: _buildBody()),
          ],
        ),
      ),
    );
  }

  Widget _buildBody() {
    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (!_hasSearched) {
      return const Center(child: Text('Find your favorite songs'));
    }

    if (_results.isEmpty) {
      return const Center(child: Text('No results found.'));
    }

    return ListView.builder(
      itemCount: _results.length,
      itemBuilder: (context, index) {
        final track = _results[index];

        return TrackListTile(
          track: track,
          trailingIcon: Icons.add_circle_outline,
          onTapTrailing: () async {
            final controller = context.read<RoomController>();
            final currentRoom = controller.currentRoom;

            if (currentRoom != null) {
              try {
                await controller.addTrack(currentRoom, track);
                if (context.mounted) {
                  UiUtils.showSuccess(
                    context,
                    '${track.title} added to the queue',
                  );
                }
              } catch (e) {
                if (context.mounted) {
                  UiUtils.showError(
                    context,
                    'Failed to add the song to the queue.',
                  );
                }
              }
            } else {
              UiUtils.showError(context, 'Join a room to add a song.');
            }
          },
        );
      },
    );
  }
}
