<p align="center">
  <img src="assets/icon-256.png" width="120" alt="adguard" />
</p>

# adguard

AdGuard Home is a network-wide DNS server that blocks ads and trackers for every device pointed at it.

A first-party [orca](https://github.com/argyle-labs/orca) plugin (service-backend).

This repo is **self-contained** — the steps below run adguard **by hand, without orca**. orca automates exactly this (same image, ports, and data) through one generic surface.

---

## Run it without orca

### Docker / Podman

```yaml
# compose.yml
services:
  adguard:
    image: adguard/adguardhome:latest
    container_name: adguard
    restart: unless-stopped
    ports:
      - "53:53/tcp"   # DNS
      - "53:53/udp"   # DNS
      - "3000:3000/tcp"   # first-run setup UI (admin moves to :80)
    volumes:
      - ./work:/opt/adguardhome/work   # runtime data, stats, query log
      - ./conf:/opt/adguardhome/conf   # AdGuardHome.yaml
```

```sh
docker compose up -d
```

Podman: the same file with `podman-compose up -d`.

### Ports & data

| | |
|---|---|
| Default port | `3000` |
| Upstream | <https://github.com/AdguardTeam/AdGuardHome> |
| Operator notes | [adguard.md](docs/adguard.md) |


### Backup & restore

Back up the config/data volume(s) above — that's the whole service state (stop the container first for a clean copy). Restore by putting them back and starting it.

> With orca this is **`service.backup` / `service.restore`** — location-agnostic (docker / podman / lxc / vm), one command regardless of where adguard runs. No per-service backup script.

## With orca

orca drives this plugin through the single generic `service.*` surface — no per-plugin tools:

```sh
orca service.deploy adguard      # render + launch on any supported runtime
orca service.status adguard      # health + rich diagnostics (typed payload)
orca service.backup adguard      # location-agnostic backup (tar; PBS on Proxmox)
orca service.configure adguard   # apply config via the upstream API
```

## Layout

- `src/` — the plugin (pure Rust): the `ServiceBackend` descriptor + `configure` / `status`.
- `docs/` — standalone operator notes.
- [CAPABILITIES.md](CAPABILITIES.md) — the service-backend contract checklist.
- `assets/` — plugin icon.
