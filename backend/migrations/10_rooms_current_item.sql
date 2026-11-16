-- The queue entry being played. A track can be queued several times, so a
-- NextTrack names the entry it wants to skip, not the track.
ALTER TABLE rooms ADD COLUMN IF NOT EXISTS current_item UUID;

-- A track already playing gets an identifier of its own.
UPDATE rooms SET current_item = gen_random_uuid()
WHERE current_track IS NOT NULL AND current_item IS NULL;

ALTER TABLE rooms DROP CONSTRAINT IF EXISTS current_item_matches_track;
ALTER TABLE rooms ADD CONSTRAINT current_item_matches_track
    CHECK ((current_item IS NULL) = (current_track IS NULL));
