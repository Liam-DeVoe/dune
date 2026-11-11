# treachery-online

Self-hosted copy of [treachery.online](https://github.com/ronaldossendrijver/treachery.online), running at https://treachery.exe.xyz/. The Rust port (the server in `rust/server` and the React client in `web/`) runs next to it at https://treachery.exe.xyz/beta/, with its own database.

- `treachery-online/` is the game source. It was brought in with `git subtree`, so upstream history is preserved and edits are committed here like any other file.
- `deploy/` is everything needed to rebuild the box from scratch. The VM is disposable; this repo is the source of truth.

## Day to day

Edit code under `treachery-online/`, then:

```
deploy/deploy.sh
```

That rsyncs the tree to the VM, runs `dotnet publish` there, and restarts the service. About a minute. The sqlite database on the VM is kept.

To update the beta:

```
deploy/deploy-beta.sh
```

That builds `web/` locally with base `/beta/`, rsyncs it and `rust/` to the VM, builds the server there with cargo (about 3 minutes from scratch, seconds when nothing changed), and restarts the beta service. The beta reads the art and CSS from the C# app's published `wwwroot`, so run `deploy.sh` first on a new VM.

exe.dev forwards https://treachery.exe.xyz/ to Caddy on port 8080 of the VM ([`deploy/Caddyfile`](deploy/Caddyfile)). Caddy sends `/beta/*` to the Rust server on port 8001, with the `/beta` prefix removed, and everything else to the C# server on port 8000. The beta sends no mail, the same as the C# server, because `GameEndEmailUsername` is empty in both.

## Rebuilding the box

```
deploy/provision.sh
```

Creates the VM on exe.dev, installs the .NET 10 SDK, Rust and Caddy, installs the systemd units and the Caddyfile, deploys both versions, and creates an `admin` account in each database. The passwords are written to `deploy/.admin-password` and `deploy/.admin-password-beta`, which are gitignored. Last, it points the public URL at Caddy, once both versions answer through it. Overrides for the VM name, ssh key, and exe.dev account are at the top of `deploy/env.sh`.

## Pulling upstream changes

```
git subtree pull --prefix=treachery-online https://github.com/ronaldossendrijver/treachery.online.git master --squash
```

## On the VM

| What | Where |
|---|---|
| service | `systemctl status treachery`, logs via `journalctl -u treachery` (`just logs`) |
| source | `/opt/treachery/src` |
| published app | `/opt/treachery/app` |
| database | `/opt/treachery/data/treachery.db` |
| beta service | `systemctl status treachery-beta`, logs via `journalctl -u treachery-beta` (`just logs-beta`) |
| beta source and cargo build | `/opt/treachery/rust-src` |
| beta server binary and web build | `/opt/treachery/beta` |
| beta database | `/opt/treachery/data-beta/treachery.db` |
| reverse proxy | `systemctl status caddy`, config in `/etc/caddy/Caddyfile` |
