# Star Chart App

A simple school reward system with centralised server. Teacher log in, award (or cash in reward) stars for students, and every connected cleint sees updates in real time via websocket.

## Stage

- [x] **Development** <- current
- [ ] Testing 
- [ ] Prodduction

## Tech Stack
**Backend:** Rust, Axum, SQLx (SQLite, WAL), Argon2 (password hashing)

**Frontend:** React, typescript, jotai


## Getting Started

### Run code
To run backend run. Go to .../StarChart/star_chart 

```bash
cargo run               (debug)
cargo run --release     (Runs optimised release build)
cargo build             (Build)
```

The server listens on `127.0.0.1:3000`. The SQLite database (`app.db`) is created automatically and migrations run on startup.

## Data Model

| Table | Columns |
| --- | --- |
| `students` | `id` (incremented), `name`, `ks`, `total_star` |
| `teachers` | `id` (incremented), `name`, `hashed_password` |
| `activites` | `id` (UUID v7), `student_id`, `teacher_id`, `reward_description`, `star_diff`, `timestamp` |

> Please look at migrations for source of truth

