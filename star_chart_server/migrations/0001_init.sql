CREATE TABLE students (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    ks          TEXT NOT NULL CHECK (ks IN ('ks3', 'ks4')),
    total_stars INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE teachers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL UNIQUE,
    hashed_password TEXT NOT NULL
);

CREATE TABLE activities (
    id                 TEXT PRIMARY KEY NOT NULL,
    student_id         INTEGER NOT NULL REFERENCES students(id),
    teacher_id         INTEGER NOT NULL REFERENCES teachers(id),
    reward_description TEXT NOT NULL,
    star_diff          INTEGER NOT NULL,
    timestamp          TEXT NOT NULL
);
