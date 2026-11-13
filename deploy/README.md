# treachery-online

Self-hosted copy of [treachery.online](https://github.com/ronaldossendrijver/treachery.online), running at https://treachery.exe.xyz/. The site is the Rust server (`rust/server`) and the React client (`web/`). The original C# code is in `treachery-online/`; it no longer runs anywhere and is kept as the reference for the Rust port until the port's plan removes it.

- `treachery-online/` is the original C# source. It was brought in with `git subtree`, so upstream history is preserved. The site uses nothing from it: the art, fonts, scripts and PDFs are in `web/assets`, the SCSS in `web/styles`.
- `deploy/` is everything needed to rebuild the box from scratch. The VM is disposable; this repo is the source of truth.

## Day to day

```
just deploy
```

That runs `deploy/deploy.sh`. It builds `web/` here, compiles the stylesheet with Dart Sass, and rsyncs the web build, the assets and `rust/` to the VM. Then it builds the server there with cargo (about 3 minutes from scratch, under a minute after) and restarts the service. If `deploy/treachery.service` or `deploy/Caddyfile` changed, it installs them too. The database on the VM is never touched.

exe.dev forwards https://treachery.exe.xyz/ to Caddy on port 8080 of the VM ([`deploy/Caddyfile`](Caddyfile)), which proxies everything to the Rust server on port 8000. `/beta/...`, where the Rust site ran before it replaced the C# one, redirects to the same path at the root. The server sends no mail, because no SMTP user is set. Its settings are environment variables in `deploy/treachery.service`, described in `rust/server/src/settings.rs`; [rust/docs/server.md](../rust/docs/server.md) describes the server.

## Rebuilding the box

```
just provision
```

Creates the VM on exe.dev, installs Rust, Caddy and sqlite3, deploys, and creates an `admin` account in the new database. The password is written to `deploy/.admin-password`, which is gitignored. Last, it points the public URL at Caddy. Overrides for the VM name, ssh key, and exe.dev account are at the top of `deploy/env.sh`.

A new box starts with an empty database. To keep the accounts, copy the old box's database to `/opt/treachery/db/treachery.db` (with `sqlite3 old.db ".backup new.db"`) before the first deploy. The server brings an older database, the C# server's included, to its current schema when it starts.

## Pulling upstream changes

```
git subtree pull --prefix=treachery-online https://github.com/ronaldossendrijver/treachery.online.git master --squash
```

## On the VM

| What | Where |
|---|---|
| service | `systemctl status treachery`, logs via `journalctl -u treachery` (`just logs`) |
| source and cargo build | `/opt/treachery/rust-src` |
| server binary, web build and assets | `/opt/treachery/server` |
| database | `/opt/treachery/db/treachery.db` |
| database backups | `~/backups` |
| reverse proxy | `systemctl status caddy`, config in `/etc/caddy/Caddyfile` |

Left over from before the move to Rust (2026-11-11), not used by anything: the C# app and its database in `/opt/treachery/{src,app,app.old,data}` (the C# service is disabled), and the beta's build and database in `/opt/treachery/{beta,data-beta}`.
