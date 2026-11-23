# Deployment

How to run the server on a small Linux VPS with HTTPS.
Every name, domain and address below is a placeholder: replace `starchart.example.com` with your own domain.

> **Never commit secrets.** The JWT secret, the admin code, the database and its backups only ever live on the server. `.gitignore` already excludes `*.db`, `*.env` and `logs/`.

## Overview

```
Internet
   │  443 (HTTPS and WebSocket)
   ▼
Caddy: gets and renews the HTTPS certificate, adds X-Forwarded-For
   │  127.0.0.1:3000
   ▼
star_chart_server: run by systemd as its own user
   │
   ▼
/var/lib/star-chart/app.db and logs/      /var/backups/star-chart/ (nightly copy)
```

- The server only listens on `127.0.0.1:3000`, so the internet can't reach it except through Caddy.
- Caddy handles HTTPS. The app connects to `https://starchart.example.com` and `wss://starchart.example.com/ws`.
- You need a **VPS**. Shared or web hosting can't keep a program running with open WebSockets.

## 1. Prepare the VPS

Create the VPS with an Ubuntu LTS image and add your SSH key. Then, logged in over SSH:

```bash
# Updates, and security updates installed automatically from now on
sudo apt update && sudo apt upgrade -y
sudo apt install -y unattended-upgrades sqlite3

# Firewall: SSH and web traffic only. Port 3000 stays closed.
sudo ufw allow OpenSSH
sudo ufw allow 80
sudo ufw allow 443
sudo ufw enable
```

Log in with SSH keys only: set `PasswordAuthentication no` in `/etc/ssh/sshd_config`, then `sudo systemctl restart ssh`. Check that key login works in a second terminal before closing the first one.

## 2. Point the domain at it

In your DNS settings, add an `A` record for `starchart.example.com` with the VPS's IP address. Caddy can't get a certificate until this works, which can take a few minutes.

## 3. Create a user and folders

The server runs as its own user with no login shell, and can only write to its data folder.

```bash
sudo useradd --system --home /var/lib/star-chart --shell /usr/sbin/nologin starchart
sudo install -d -m 700 -o starchart -g starchart /var/lib/star-chart
sudo install -d -m 700 -o root -g root /var/backups/star-chart
```

## 4. Secrets

Create the environment file, readable only by root (systemd reads it before starting the server):

```bash
sudo install -m 600 -o root -g root /dev/null /etc/star-chart.env
sudo nano /etc/star-chart.env
```

Put in two long random values. Generate each with `openssl rand -base64 48`:

```
JWT_SECRET=<a long random value>
ADMIN_PASSWORD=<a different long random value>
```

- `JWT_SECRET` signs login tokens. Changing it logs everyone out.
- `ADMIN_PASSWORD` is the admin code needed to create teacher accounts. Keep it in a password manager.

A release build refuses to start if either is missing, or if `JWT_SECRET` is shorter than 32 characters. Debug builds fall back to built-in values that are public in this repo, so **always deploy a release build**.

## 5. Build on your own computer

Don't build on the VPS: the release profile (LTO, one codegen unit) is very slow and memory-hungry on a single core. Cross-compile a single static binary instead.

Check the VPS's CPU type with `uname -m`. For `x86_64`:

```bash
# One-time setup
brew install zig                 # or your OS's package manager
cargo install cargo-zigbuild
rustup target add x86_64-unknown-linux-musl

# Build (from the repo root)
cargo zigbuild --release -p star_chart_server --target x86_64-unknown-linux-musl
```

For `aarch64`, use `aarch64-unknown-linux-musl` instead. The database migrations are compiled into the binary, so it's the only file to copy:

```bash
scp target/x86_64-unknown-linux-musl/release/star_chart_server you@starchart.example.com:/tmp/
ssh you@starchart.example.com 'sudo install -m 755 /tmp/star_chart_server /usr/local/bin/ && rm /tmp/star_chart_server'
```

## 6. Run it with systemd

Create `/etc/systemd/system/star-chart.service`:

```ini
[Unit]
Description=Star Chart server
After=network.target

[Service]
User=starchart
Group=starchart
WorkingDirectory=/var/lib/star-chart
EnvironmentFile=/etc/star-chart.env
ExecStart=/usr/local/bin/star_chart_server
Restart=on-failure

# Sandboxing: the server can only write to its own folder
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadWritePaths=/var/lib/star-chart

[Install]
WantedBy=multi-user.target
```

`WorkingDirectory` decides where `app.db` and `logs/` are created. Then:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now star-chart
sudo systemctl status star-chart
```

`systemctl stop` and `restart` send SIGTERM, which shuts the server down gracefully and closes open WebSockets.

## 7. HTTPS with Caddy

Install Caddy with the official instructions for Ubuntu: <https://caddyserver.com/docs/install>. Then replace `/etc/caddy/Caddyfile` with:

```
starchart.example.com {
    reverse_proxy 127.0.0.1:3000
}
```

```bash
sudo systemctl reload caddy
```

Caddy gets and renews the certificate itself, passes WebSockets through, and sets `X-Forwarded-For` to the client's real address.

**Why `X-Forwarded-For` matters.** Behind Caddy, every connection reaches the server from `127.0.0.1`. The server reads the client's address from the **last** entry of `X-Forwarded-For` instead, so the rate limits apply per client. It only trusts that header on connections from `127.0.0.1`, because anyone else could put a fake address in it. This is why the server must only listen on localhost and be reached through the proxy.

## 8. Create the first teacher

From your own computer, with the admin code from step 4. The leading space keeps the command out of your shell history in bash, and in zsh after `setopt HIST_IGNORE_SPACE`:

```bash
 curl -i -X POST https://starchart.example.com/create_login \
   -H 'Content-Type: application/json' \
   -d '{"username":"teacher1","password":"<at least 8 characters>","admin_code":"<admin code>"}'
```

Expect `201 Created`. The app's test page can do the same.

## 9. Backups

The database holds password hashes, so the backups stay readable by root only. Create `/etc/cron.daily/star-chart-backup`:

```sh
#!/bin/sh
set -e
dir=/var/backups/star-chart
# The online backup is safe while the server is running
sqlite3 /var/lib/star-chart/app.db ".backup '$dir/app-$(date +%F).db'"
chmod 600 "$dir"/app-*.db
# Keep two weeks
find "$dir" -name 'app-*.db' -mtime +14 -delete
```

```bash
sudo chmod 755 /etc/cron.daily/star-chart-backup
sudo /etc/cron.daily/star-chart-backup    # test it once
```

Backups on the same machine don't help if the machine is lost. Also turn on your host's VPS snapshots, or copy the backups somewhere else.

**Restoring:** stop the server, copy a backup over `/var/lib/star-chart/app.db` (owned by `starchart`, mode 600), delete any `app.db-wal` and `app.db-shm` next to it, and start the server again.

## Updating

Build and copy the new binary as in step 5, then:

```bash
sudo systemctl restart star-chart
```

New migrations run when the server starts. Take a backup first if the update changes the database.

## Logs

- `journalctl -u star-chart`: startup errors, such as a missing secret.
- `/var/lib/star-chart/logs/`: the server's own log, one file per day, the last 7 kept. It includes client IP addresses (in rate limit warnings), so don't share it publicly.

## Pointing the app at the server

Start the app with `STAR_CHART_SERVER=https://starchart.example.com`. The app's WebSocket library needs a TLS feature turned on for `wss://` first (see [todo.md](todo.md)).

## Rate limits

| Limit | Default | Why |
| --- | --- | --- |
| Requests per client address | 30 per minute | Stops floods. Loose, because a whole school usually shares one public address. |
| Wrong passwords per username | 5, then one more every 3 minutes | Stops password guessing. A correct login doesn't count. |
| Wrong admin codes, whole server | 5, then one more every 3 minutes | Stops guessing the admin code, from any number of addresses. |

The limits are constants at the top of `star_chart_server/src/main.rs`. The per-username limit has a trade-off: someone who knows a teacher's username can keep that account locked by sending wrong passwords. The teacher's password stays safe, and the lock lifts a few minutes after the wrong attempts stop.
