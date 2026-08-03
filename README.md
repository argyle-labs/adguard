<p align="center">
  <img src="assets/icon-256.png" width="120" alt="adguard" />
</p>

# adguard

AdGuard Home is a network-wide DNS server that blocks ads and trackers for every device pointed at it.

A first-party [orca](https://github.com/argyle-labs/orca) plugin (service-backend).

This repo is **self-contained** — the steps below run adguard **by hand, without orca**. orca automates exactly this (same image, ports, and data) through one generic surface.

---

## Run it without orca

### Docker Compose

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

### Other runtimes

**Podman** — the compose above works with `podman compose up -d`, or run it directly:

```sh
podman run -d --name adguard --restart unless-stopped \
    -p 53:53/tcp \
    -p 53:53/udp \
    -p 3000:3000/tcp \
    -v ./work:/opt/adguardhome/work \
    -v ./conf:/opt/adguardhome/conf \
    adguard/adguardhome:latest
```

**LXC** — on a container-capable LXC (e.g. a Proxmox LXC with nesting enabled) run the same image via Docker/Podman as above, or install adguard from upstream directly on the guest: <https://github.com/AdguardTeam/AdGuardHome>.

**VM** — install adguard from upstream (<https://github.com/AdguardTeam/AdGuardHome>) or run the same container image inside the VM; expose port `53`.

**Unraid** — add via *Community Applications*, or *Docker → Add Container* with image `adguard/adguardhome:latest`, port `53`, and the volume paths above.

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
orca service.status adguard      # health + rich diagnostics (typed payload) — planned, not yet implemented
orca service.backup adguard      # location-agnostic backup (tar; PBS on Proxmox)
orca service.configure adguard   # apply config via the upstream API — planned, not yet implemented
```

> Note: `service.status` and `service.configure` are planned but not yet implemented — both currently return an unimplemented error. See `roadmap.md`.

## Layout

- `src/` — the plugin (pure Rust): the `ServiceBackend` descriptor + `configure` / `status`.
- `docs/` — standalone operator notes.
- [CAPABILITIES.md](CAPABILITIES.md) — the service-backend contract checklist.
- `assets/` — plugin icon.
