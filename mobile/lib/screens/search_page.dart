import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/room_controller.dart';
import '../core/theme.dart';
import '../core/utils/ui_utils.dart';
import '../models/track.dart';
import '../services/music_repository.dart';
import '../widgets/track_list_tile.dart';

class SearchPage extends StatefulWidget {
  const SearchPage({super.key, this.musicRepository = const MusicRepository()});

  final MusicRepository musicRepository;

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final TextEditingController _searchController = TextEditingController();
  bool _isLoading = false;
  bool _hasSearched = false;
  List<Track> _results = [];

  /// Number of the latest search; answers to older ones are dropped.
  int _searchRequest = 0;

  Future<void> _performSearch(String query) async {
    final request = ++_searchRequest;

    if (query.trim().isEmpty) {
      setState(() {
        _results = [];
        _isLoading = false;
        _hasSearched = false;
      });
      return;
    }

    setState(() {
      _isLoading = true;
      _hasSearched = true;
    });

    try {
      final results = await widget.musicRepository.searchTracks(query);
      if (!mounted || request != _searchRequest) return;
      setState(() {
        _results = results;
        _isLoading = false;
      });
    } catch (e) {
      if (!mounted || request != _searchRequest) return;
      setState(() {
        _isLoading = false;
      });
      UiUtils.showError(context, 'Search failed: $e');
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

    if (!_hasSearched || _results.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.search_outlined,
              size: 64,
              color: Theme.of(context).colorScheme.secondary,
            ),
            const SizedBox(height: 8),
            Text(
              !_hasSearched ? 'Find your favorite songs' : 'No results found.',
              textAlign: TextAlign.center,
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 8),
            const Text(
              'Try searching for a song.',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 14, color: Colors.grey),
            ),
          ],
        ),
      );
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

            if (controller.currentRoom != null) {
              try {
                await controller.addTrack(track);
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
