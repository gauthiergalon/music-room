import 'package:flutter/material.dart';
import '../models/track.dart';
import '../core/theme.dart';

class TrackListTile extends StatelessWidget {
  final Track track;
  final VoidCallback onTapTrailing;
  final IconData trailingIcon;

  const TrackListTile({
    super.key,
    required this.track,
    required this.onTapTrailing,
    this.trailingIcon = Icons.add_circle_outline,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return ListTile(
      contentPadding: const EdgeInsets.symmetric(
        horizontal: AppTheme.spacingMd,
        vertical: 0.0,
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
      title: Text(track.title, maxLines: 1, overflow: TextOverflow.ellipsis),
      subtitle: Text(
        track.artist,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      trailing: IconButton(icon: Icon(trailingIcon), onPressed: onTapTrailing),
    );
  }
}
