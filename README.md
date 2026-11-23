# Star Chart Server

The central server for a school reward system. Teachers log in and award stars to students, or cash stars in for rewards. Every connected client sees the changes in real time over a WebSocket.

## Stage

- [x] **Development** <- current
- [ ] Testing
- [ ] Production

## Tech Stack

**Frontend:** React, typescript, jotai


## Getting Started

### Prerequisites

- **Rust 1.85 or newer.** The server and protocol crates use edition 2024. Install it with [rustup](https://rustup.rs).
- **Node.js and npm**, for the frontend.
- **The Tauri system dependencies** for your OS: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

### Project layout

The repo is a Cargo workspace with three crates:

| Folder | What it is |
| --- | --- |
| `star_chart_server/` | The server (Axum + SQLite) |
| `starChartFrontend/` | The desktop app (Tauri + React). Its Rust code is in `src-tauri/`. |
| `star_chart_protocol/` | The message and data types shared by the server and the app |

Every crate builds into one `target/` folder at the repo root. The build profiles are also set in the root `Cargo.toml`.

### Running the server

Run the server from the `star_chart_server` folder:

```bash
cd star_chart_server
cargo run               # debug build
cargo run --release     # optimised release build
```

The server listens on `127.0.0.1:3000`. Only connections from the same machine can reach it.

The SQLite database `app.db` is created in the current working directory if it doesn't exist. Migrations are compiled into the binary and run on startup. You can also run `cargo run -p star_chart_server` from the repo root, but that creates a separate `app.db` and `logs/` folder there.

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

### Running the app

Start the server first. Then, in a second terminal:

```bash
cd starChartFrontend
npm install             # first time only
npm run tauri dev       # build and open the app in development mode
```

`tauri dev` starts the Vite dev server on `localhost:1420` and shows it in the app window. React changes reload instantly. Changes to the Rust code in `src-tauri/` rebuild and restart the app.

To build an installable app, run `npm run tauri build`. The installers are written to `target/release/bundle/` at the repo root.

### Checking the whole workspace

From the repo root:

```bash
cargo check --workspace     # type-check every crate
cargo build --workspace     # build every crate
cargo clippy --workspace    # lint every crate
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
- [Deployment](docs/deployment.md): running the server on a VPS with HTTPS, backups and rate limits
- [Todo](docs/todo.md)
