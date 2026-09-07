<p align="center">
  <img src="assets/icon-256.png" width="120" alt="adguard" />
</p>

# adguard

AdGuard Home is a network-wide DNS server that blocks ads and trackers for every device pointed at it.

A first-party [orca](https://github.com/argyle-labs/orca) plugin: first-class CRUD over an already-running AdGuard Home instance's DNS rewrites (plus a status read) over its REST API — so you stop hand-editing `AdGuardHome.yaml`.

This repo is **self-contained** — the steps below run adguard **by hand, without orca**. orca then drives the running instance's DNS rewrites through the tools below.

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

## With orca

Register a running AdGuard Home instance as an endpoint, then drive its DNS rewrites over the REST API:

```sh
# Register the instance (routes are an ordered, reachable-first fallback list).
orca adguard.create --name home --username admin \
    --route lan=http://10.0.0.5:80 --insecure false --enabled true
orca adguard.list                                    # registered endpoints
orca adguard.status --name home                      # version + running/protection

# DNS rewrites
orca adguard.rewrite.list --name home
orca adguard.rewrite.add    --name home --domain service.example.com --answer 10.0.0.9
orca adguard.rewrite.set    --name home --domain service.example.com --answer 10.0.0.9  # idempotent upsert
orca adguard.rewrite.delete --name home --domain service.example.com --answer 10.0.0.9
```

The admin password lives in orca's abstract secrets domain (`adguard.<endpoint>.password`), never a plaintext column. Auth is HTTP Basic; `--insecure true` skips TLS verification for a self-signed https front-end.

## Layout

- `src/lib.rs` — the AdGuard Home REST client (`Config`, rewrite/status ops).
- `src/tools.rs` — the `#[endpoint_resource]` registry + `adguard.*` tools.
- `src/main.rs` — the `Plugin` builder entrypoint.
- `docs/` — standalone operator notes.
- `assets/` — plugin icon.
