import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/services/queue_order.dart';

void main() {
  group('computeReorderPosition', () {
    test('a single item gets position 0', () {
      expect(computeReorderPosition([42], 0), 0);
    });

    test('moved to the front, it sorts before the first item', () {
      // The item with position 30 was moved to the front of [10, 20].
      final position = computeReorderPosition([30, 10, 20], 0);
      expect(position, lessThan(10));
    });

    test('moved to the end, it sorts after the last item', () {
      final position = computeReorderPosition([20, 30, 10], 2);
      expect(position, greaterThan(30));
    });

    test('moved to the middle, it sorts between its neighbours', () {
      final position = computeReorderPosition([10, 30, 20], 1);
      expect(position, greaterThan(10));
      expect(position, lessThan(20));
    });

    test('works with negative and fractional neighbours', () {
      final position = computeReorderPosition([-90, 0, -89.5], 1);
      expect(position, greaterThan(-90));
      expect(position, lessThan(-89.5));
    });
  });
}
