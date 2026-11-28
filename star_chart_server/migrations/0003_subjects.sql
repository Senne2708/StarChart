-- Every star change now belongs to a class, and balances are kept per class.
-- The activities so far were test data with no class, so start again from empty.
DROP TABLE activities;

CREATE TABLE activities (
    id                 TEXT PRIMARY KEY NOT NULL,
    student_id         INTEGER NOT NULL REFERENCES students(id),
    teacher_id         INTEGER NOT NULL REFERENCES teachers(id),
    reward_description TEXT NOT NULL,
    star_diff          INTEGER NOT NULL,
    timestamp          TEXT NOT NULL,
    kind               TEXT NOT NULL CHECK (kind IN ('reward', 'take_away', 'cash_in')),
    subject            TEXT NOT NULL CHECK (subject IN (
        'art', 'ilp', 'maths', 'english', 'humanities', 'citizenship', 'vocational', 'options'
    ))
);

CREATE INDEX activities_student_subject ON activities (student_id, subject);

-- Balances are now worked out from activities instead of stored
ALTER TABLE students DROP COLUMN total_stars;
