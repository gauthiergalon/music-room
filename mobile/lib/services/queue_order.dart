/// Gap left between two queue positions when an item is moved to an end.
const double queuePositionStep = 100;

/// Position to give the item that now sits at [index] in a reordered queue.
///
/// [positions] are the positions of the queue *after* the move, in display
/// order; the value at [index] is the moved item's old position and is
/// ignored. The result sorts strictly between its new neighbours.
double computeReorderPosition(List<double> positions, int index) {
  if (positions.length <= 1) return 0;
  if (index <= 0) return positions[1] - queuePositionStep;
  if (index >= positions.length - 1) {
    return positions[positions.length - 2] + queuePositionStep;
  }
  return (positions[index - 1] + positions[index + 1]) / 2;
}
