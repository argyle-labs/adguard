# adguard plugin — roadmap

Intended functionality for the orca adguard plugin, beyond today's deploy config.
Not yet implemented — captured so the intent isn't lost.

## Control & configure AdGuard Home

The plugin should let orca **control and configure** a running AdGuard Home
instance via its HTTP API (`/control/*`):

- **DNS rewrites** — list/add/remove local overrides (e.g. `*.<domain> → <proxy-ip>`).
  Adding a service subdomain should be one orca call.
- **Filtering rules / `user_rules`** — manage custom rules, including rebind-
  protection exceptions (e.g. `@@||plex.direct^$important`, `@@||plex.tv^$important`)
  needed for services that publish private-IP DNS records.
- **Upstream DNS + bootstrap** — configure upstreams (local resolver → public),
  and the DNS chain (AdGuard → local Unbound → upstream).
- **Blocklists** — enable/disable filter lists (e.g. HaGeZi DNS Rebind Protection).
- **Query log / stats** — read for observability.
- **Status/health** — protection on/off, running state.

Reference: the DNS chain, rewrites, and rebind exceptions are documented in
[`adguard.md`](adguard.md).
