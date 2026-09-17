# treachery-online

Self-hosted copy of [treachery.online](https://github.com/ronaldossendrijver/treachery.online), running at https://treachery.exe.xyz/.

- `treachery-online/` is the game source. It was brought in with `git subtree`, so upstream history is preserved and edits are committed here like any other file.
- `deploy/` is everything needed to rebuild the box from scratch. The VM is disposable; this repo is the source of truth.

## Day to day

Edit code under `treachery-online/`, then:

```
deploy/deploy.sh
```

That rsyncs the tree to the VM, runs `dotnet publish` there, and restarts the service. About a minute. The sqlite database on the VM is kept.

## Rebuilding the box

```
deploy/provision.sh
```

Creates the VM on exe.dev, installs the .NET 10 SDK, installs the systemd unit, deploys, and creates the `admin` account. The admin password is written to `deploy/.admin-password`, which is gitignored. Overrides for the VM name, ssh key, and exe.dev account are at the top of `deploy/env.sh`.

## Pulling upstream changes

```
git subtree pull --prefix=treachery-online https://github.com/ronaldossendrijver/treachery.online.git master --squash
```

## On the VM

| What | Where |
|---|---|
| service | `systemctl status treachery`, logs via `journalctl -u treachery` |
| source | `/opt/treachery/src` |
| published app | `/opt/treachery/app` |
| database | `/opt/treachery/data/treachery.db` |
