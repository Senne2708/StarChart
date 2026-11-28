# Todo

Roughly in the order to do them.

## Server
- [x] Take the teacher for `add_activity` from the login token instead of the message.
- [x] Read the client address from `X-Forwarded-For` behind the reverse proxy, and limit wrong passwords per username and wrong admin codes per server.

## Desktop app (Tauri, `src-tauri/`)
- [x] `error.rs`, `auth.rs`, `server.rs`, `state.rs`, `commands.rs`: log in, open the WebSocket, one command per message, broadcasts sent to the UI as events.
- [x] Server URL set with `STAR_CHART_SERVER` (defaults to `http://127.0.0.1:3000`).
- [ ] A settings screen for the server URL, instead of an environment variable.
- [ ] Move `LoginPayload`/`LoginResponse` into `star_chart_protocol`, so the server and the app share them.
- [x] Enable TLS on `tokio-tungstenite` (`native-tls`) so `wss://` works. Release builds refuse plain `http://` except to localhost.

## Frontend (React)
- [ ] Move `jotai` from the root `package.json` into `starChartFrontend/package.json`, and delete the root one.
- [ ] Make `atoms.tsx` match the protocol types: number ids, `total_stars`, `ks`, and lowercase `"ks3"`/`"ks4"`.
- [ ] Add a login screen.
- [ ] Replace the mock data with live data. The leaderboard +/- buttons should send `add_activity` with a reward description.
- [ ] Handle `student_added`, `activity_added` and `resync`.
- [ ] Hide the delete-student button until the server supports it.
- [x] Set a Content Security Policy in `tauri.conf.json`.
- [ ] Replace the Tauri template names (`productName`, `identifier`, Cargo `description` and `authors`).

## Deployment
See [deployment.md](deployment.md). No code changes are needed: systemd's `WorkingDirectory` sets where the database and logs go, and Caddy handles TLS.
- [ ] Set up the VPS, Caddy, the systemd service and nightly backups.
