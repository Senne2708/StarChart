# Todo

Roughly in the order to do them.

## Server
- [ ] Behind a reverse proxy (see Deployment), every request comes from the proxy's address, so the rate limit would need to read `X-Forwarded-For` instead.

## Desktop app (Tauri, `src-tauri/`)
- [x] `error.rs`, `auth.rs`, `server.rs`, `state.rs`, `commands.rs`: log in, open the WebSocket, one command per message, broadcasts sent to the UI as events.
- [x] Server URL set with `STAR_CHART_SERVER` (defaults to `http://127.0.0.1:3000`).
- [ ] A settings screen for the server URL, instead of an environment variable.
- [ ] Move `LoginPayload`/`LoginResponse` into `star_chart_protocol`, so the server and the app share them.
- [ ] Enable a TLS feature on `tokio-tungstenite` (e.g. `rustls-tls-webpki-roots`) before using `wss://`.

## Frontend (React)
- [ ] Move `jotai` from the root `package.json` into `starChartFrontend/package.json`, and delete the root one.
- [ ] Make `atoms.tsx` match the protocol types: number ids, `total_stars`, `ks`, and lowercase `"ks3"`/`"ks4"`.
- [ ] Add a login screen.
- [ ] Replace the mock data with live data. The leaderboard +/- buttons should send `add_activity` with a reward description.
- [ ] Handle `student_added`, `activity_added` and `resync`.
- [ ] Hide the delete-student button until the server supports it.
- [ ] Set a Content Security Policy in `tauri.conf.json` (it is `null`).
- [ ] Replace the Tauri template names (`productName`, `identifier`, Cargo `description` and `authors`).

## Deployment
- [ ] Set up the server: environment variables for the bind address, database path and log folder.
- [ ] TLS, through a reverse proxy or `rustls`.
- [ ] Run it as a service, with database backups.
