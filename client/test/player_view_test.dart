import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/widgets/player.dart';

const _coverKey = ValueKey('player-cover');

Widget _player({Track? track}) {
  return MaterialApp(
    home: Scaffold(
      body: PlayerView(
        track: track,
        isPlaying: true,
        canControl: true,
        positionStream: const Stream.empty(),
        currentPosition: () => const Duration(seconds: 42),
        onSeek: (_) {},
        onTogglePlay: () {},
        onSkipNext: () {},
        onSkipPrevious: () {},
      ),
    ),
  );
}

final _track = Track(
  id: 1,
  title: 'A fairly long track title that may need more than one line',
  artist: 'Some Artist',
  duration: const Duration(minutes: 3, seconds: 30),
);

Future<void> _pumpAt(WidgetTester tester, Size size, {Track? track}) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(_player(track: track ?? _track));
  await tester.pump();
}

void main() {
  // Every shape the room screen can realistically take, including the
  // landscape web window that used to overflow.
  const sizes = <String, Size>{
    'phone portrait': Size(390, 760),
    'small phone portrait': Size(320, 480),
    'phone landscape': Size(800, 320),
    'short and wide browser window': Size(1280, 420),
    'desktop browser': Size(1920, 980),
    'tablet landscape': Size(1024, 700),
  };

  for (final MapEntry(key: name, value: size) in sizes.entries) {
    testWidgets('everything fits on a $name', (tester) async {
      await _pumpAt(tester, size);

      expect(tester.takeException(), isNull);

      final view = Offset.zero & size;
      for (final finder in [
        find.byKey(_coverKey),
        find.byType(Slider),
        find.byIcon(Icons.pause_rounded),
      ]) {
        expect(finder, findsOneWidget);
        final rect = tester.getRect(finder);
        expect(
          view.contains(rect.topLeft) && view.contains(rect.bottomRight),
          isTrue,
          reason: '$finder at $rect is outside $view',
        );
      }
    });
  }

  // Too small to show everything: the controls scroll instead of overflowing.
  const tinySizes = <String, Size>{
    'narrow and short split view': Size(300, 240),
    'very short window': Size(900, 160),
  };

  for (final MapEntry(key: name, value: size) in tinySizes.entries) {
    testWidgets('does not overflow on a $name', (tester) async {
      await _pumpAt(tester, size);

      expect(tester.takeException(), isNull);
      expect(find.byType(Slider), findsOneWidget);
    });
  }

  testWidgets('landscape: the cover sits beside the controls and fits the '
      'height', (tester) async {
    await _pumpAt(tester, const Size(1280, 420));

    final cover = tester.getRect(find.byKey(_coverKey));
    final slider = tester.getRect(find.byType(Slider));

    expect(cover.width, cover.height);
    expect(cover.height, lessThanOrEqualTo(420));
    // Side by side: the controls start to the right of the cover.
    expect(slider.left, greaterThan(cover.right));
    // The controls do not stretch across a wide window.
    expect(slider.width, lessThanOrEqualTo(PlayerView.maxControlsWidth));
  });

  testWidgets('portrait: the cover sits above the controls', (tester) async {
    await _pumpAt(tester, const Size(390, 760));

    final cover = tester.getRect(find.byKey(_coverKey));
    final slider = tester.getRect(find.byType(Slider));

    expect(cover.width, cover.height);
    expect(cover.bottom, lessThan(slider.top));
    expect(cover.width, lessThanOrEqualTo(390));
  });

  testWidgets('the cover never exceeds its maximum size on a huge screen', (
    tester,
  ) async {
    await _pumpAt(tester, const Size(2560, 1440));

    final cover = tester.getSize(find.byKey(_coverKey));
    expect(cover.width, PlayerView.maxCoverSize);
    expect(cover.height, PlayerView.maxCoverSize);
  });

  testWidgets('the cover is dropped when there is no room for it', (
    tester,
  ) async {
    await _pumpAt(tester, const Size(300, 240));

    expect(find.byKey(_coverKey), findsNothing);
    // The controls are still there.
    expect(find.byType(Slider), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('shows a placeholder when no track is playing', (tester) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(1280, 600);
    addTearDown(tester.view.reset);
    await tester.pumpWidget(_player());
    await tester.pump();

    expect(find.text('No track'), findsOneWidget);
    expect(find.byKey(_coverKey), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
