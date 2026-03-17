import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/controllers/room_controller.dart';

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

    // Simulation d'un appel API (à remplacer plus tard)
    await Future.delayed(const Duration(seconds: 1));

    if (!mounted) return;

    setState(() {
      _isLoading = false;
      // Fausses données de test
      _results = [
        Track(
          id: 1,
          title: '$query (Radio Edit)',
          artist: 'Unknown Artist',
          duration: const Duration(minutes: 3, seconds: 12),
          imageUrl: 'https://picsum.photos/seed/1/200',
        ),
        Track(
          id: 2,
          title: '$query - Remix',
          artist: 'DJ Test',
          duration: const Duration(minutes: 4, seconds: 5),
        ),
        Track(
          id: 3,
          title: '$query (Acoustic)',
          artist: 'Unknown Artist',
          duration: const Duration(minutes: 2, seconds: 45),
          imageUrl: 'https://picsum.photos/seed/3/200',
        ),
      ];
    });
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
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: 16.0,
                vertical: 8.0,
              ),
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
                  // Pour mettre à jour l'icône "clear"
                  setState(() {});
                },
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
        final theme = Theme.of(context);

        return ListTile(
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 16,
            vertical: 4,
          ),
          leading: ClipRRect(
            borderRadius: BorderRadius.circular(8),
            child: track.imageUrl != null
                ? Image.network(
                    track.imageUrl!,
                    width: 56,
                    height: 56,
                    fit: BoxFit.cover,
                  )
                : Container(
                    width: 56,
                    height: 56,
                    color: theme.colorScheme.surfaceContainerHighest,
                    child: Icon(
                      Icons.music_note,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
          ),
          title: Text(
            track.title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          subtitle: Text(
            track.artist,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          trailing: IconButton(
            icon: const Icon(Icons.add_circle_outline),
            onPressed: () {
              final controller = context.read<RoomController>();
              final currentRoom = controller.currentRoom;

              if (currentRoom != null) {
                controller.addTrack(currentRoom, track);
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text('${track.title} added to the queue'),
                    duration: const Duration(seconds: 3),
                  ),
                );
              } else {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('Join a room to add a song.'),
                    duration: Duration(seconds: 3),
                  ),
                );
              }
            },
          ),
        );
      },
    );
  }
}
