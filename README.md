# Star Chart Server

The central server for a school reward system. Teachers log in and award stars to students, or cash stars in for rewards. Every connected client sees the changes in real time over a WebSocket.

## Stage

- [x] **Development** <- current
- [ ] Testing
- [ ] Production

## Tech Stack

**Frontend:** React, typescript, jotai


## Getting Started

### Run code
To run backend run. Go to .../StarChart/star_chart 

```bash
cargo run               (debug)
cargo run --release     (Runs optimised release build)
cargo build             (Build)
```

The server listens on `127.0.0.1:3000`. Only connections from the same machine can reach it.

The SQLite database `app.db` is created in the current working directory if it doesn't exist. Migrations are compiled into the binary and run on startup.

### Configuration

Settings come from environment variables. The server does **not** load a `.env` file, so the variables must be set in the shell that starts it.

| Variable | Required | Description |
| --- | --- | --- |
| `JWT_SECRET` | Release builds | Secret used to sign login tokens. It must be at least 32 characters. Debug builds fall back to an insecure built-in key and log a warning. |
| `ADMIN_PASSWORD` | Release builds | The admin code needed to create teacher accounts (`POST /create_login`). Debug builds fall back to `debug_password` and log a warning. |
| `RUST_LOG` | No | Overrides the log filter, e.g. `RUST_LOG=info`. |

In a release build, the server stops at startup if `JWT_SECRET` or `ADMIN_PASSWORD` is missing, or if `JWT_SECRET` is too short.

```bash
JWT_SECRET="<at least 32 characters>" ADMIN_PASSWORD="<admin code>" cargo run --release
```

### Logging

- **Debug builds** log to the terminal at `debug` level.
- **Release builds** log to files in `logs/`, under the current working directory, at `info` level. A new file (`app.log.YYYY-MM-DD`) starts each day, and only the last 7 are kept.

### Creating the first teacher

No accounts exist at first. Create one with the admin code:

```bash
curl -i -X POST http://127.0.0.1:3000/create_login \
  -H 'Content-Type: application/json' \
  -d '{"username":"teacher1","password":"...","admin_code":"debug_password"}'
```

## Data Model

| Table | Columns |
| --- | --- |
| `students` | `id` (auto-increment), `name`, `ks` (`ks3` or `ks4`), `total_stars` (default 0) |
| `teachers` | `id` (auto-increment), `name` (unique, used as the login username), `hashed_password` |
| `activities` | `id` (UUID v7), `student_id` → `students`, `teacher_id` → `teachers`, `reward_description`, `star_diff`, `timestamp` (UTC) |

Foreign keys are enforced. See `star_chart_server/migrations/` for the exact schema.

## Documentation

- [API reference](docs/api.md): HTTP endpoints and the WebSocket protocol
- [Todo](docs/todo.md)
