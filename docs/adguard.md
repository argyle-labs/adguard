# AdGuard Home

DNS server with ad-blocking. Primary DNS for the entire homelab — all clients point here.

---

## Instance

| Field | Value |
|---|---|
| LXC ID | 101 |
| Host | <host> (<ip>) |
| IP | <ip> |
| OS | Debian 12 |
| CPU | 1 core |
| RAM | 512 MB |
| Disk | 4 GB (local-lvm) |
| Unprivileged | yes |
| onboot | yes |
| Admin UI | http://<ip>:3000 |
| DNS port | 53 |

---

## DNS Chain

```
Clients → AdGuard (<ip>:53)
             → OPNsense Unbound (<ip>) for local resolution
             → 1.1.1.1 / 1.0.0.1 upstream (via <vpn-provider> region-a tunnel on OPNsense)
```

OPNsense DHCP hands out `<ip>` as the DNS server for all LAN clients.

---

## Local DNS Overrides

AdGuard rewrites for internal services (set in AdGuard → Filters → DNS Rewrites):

| Domain | Target |
|---|---|
| *.<your-domain> | <ip> (<host>) |

> Add per-service entries here as subdomains are created.

### DNS Rebind Protection Exceptions

HaGeZi's DNS Rebind Protection blocklist is enabled and strips DNS answers that point to RFC1918 addresses. Plex uses hashed `*.plex.direct` hostnames that resolve to LAN IPs (e.g. `<ip-dashed>.<hash>.plex.direct → <ip>`) so clients can do trusted-cert HTTPS to the server. Without an exception, clients fall back to relay or fail with "server not reachable" / playback errors.

`user_rules` in `/opt/AdGuardHome/AdGuardHome.yaml` includes:

```yaml
- '@@||plex.direct^$important'
- '@@||plex.tv^$important'
```

Add similar `@@||<domain>^$important` entries for any other service that publishes private-IP DNS records (e.g. some self-hosted apps with vanity `.local.<service>.com` schemes).

---

## Service Management

```bash
pct enter 101   # on <host>

systemctl status AdGuardHome
systemctl restart AdGuardHome
```

---

## Backup

`/opt/AdGuardHome/AdGuardHome.yaml` is captured by `backup-configs.sh` via `pct exec 101 -- cat` and committed to the <repo> git repo. Archive is ~2.5K. Verified working 2026-04-25.

AdGuard Home does **not** support a native backup directory setting — there is no UI option for this. The app does not write backups on its own.

The <host> bind mount (`/mnt/<host>/backups/services/adguard` → `/mnt/backups` inside LXC 101) is configured but unused for backup purposes.

**Intentionally excluded:** `sessions.db` (user preference), `stats.db` and `querylog.json` (large, reconstructible).

---

## If AdGuard Goes Down

DNS fails for all clients. OPNsense Unbound is the fallback — temporarily change DHCP DNS option on OPNsense to `<ip>` until AdGuard is restored.

---

## Related

- [nginx-proxy-manager.md](nginx-proxy-manager.md) — reverse proxy that uses AdGuard wildcard DNS
- [opnsense-setup.md](../network/opnsense-setup.md) — Unbound + forwarding config
