-- What each star change was for: awarded, taken away, or cashed in for a reward
ALTER TABLE activities ADD COLUMN kind TEXT NOT NULL DEFAULT 'reward'
    CHECK (kind IN ('reward', 'take_away', 'cash_in'));

-- Older negative changes didn't record a kind. Treat them as taken away.
UPDATE activities SET kind = 'take_away' WHERE star_diff < 0;
