# AdGuard Home on a Proxmox LXC (native)

A standalone deployment: AdGuard Home running **natively** (not in Docker)
inside an **unprivileged Debian LXC** on Proxmox. It serves LAN DNS and blocks
ads/trackers at the network level. Nothing here needs orca.

> Placeholders: `<proxmox-host>` = your Proxmox node, `<ip>` = a LAN address,
> `<pool>` = your ZFS/backup pool. Pick the CT ID with
> `pvesh get /cluster/nextid` (shown as `<CTID>`); never hard-code one.

- **Ports**: 53 (DNS, tcp+udp), 3000 (initial setup wizard), 80 (web UI after setup)
- **Type**: Proxmox LXC — Debian minimal, **unprivileged**
- **Footprint**: 1 core / 512 MB RAM / 8 GB disk

A DNS server should have a **static IP** so clients (or your DHCP server's
"DNS servers" option) can point at a stable address.

---

## Step 1 — Create the LXC

```bash
pveam available | grep debian-12   # find the current template
pct create "$(pvesh get /cluster/nextid)" \
  local:vztmpl/debian-12-standard_12.7-1_amd64.tar.zst \
  --hostname adguard \
  --storage local-lvm \
  --rootfs local-lvm:8 \
  --cores 1 --memory 512 --swap 512 \
  --net0 name=eth0,bridge=vmbr0,ip=10.0.0.201/24,gw=10.0.0.1 \
  --features nesting=1,keyctl=1 \
  --unprivileged 1 \
  --onboot 1
```

A full sample config is in [`lxc/adguard.conf.example`](../lxc/adguard.conf.example)
— it also includes a `mp0` backup bind mount. Copy the fields you want into
`/etc/pve/lxc/<CTID>.conf` on `<proxmox-host>` (the CT must be stopped to edit).

> If `systemd-resolved` on the host is holding port 53, that's a *host* concern,
> not this CT — the LXC has its own network namespace.

## Step 2 — Install AdGuard Home

```bash
pct start <CTID>
pct enter <CTID>

apt-get update && apt-get upgrade -y
apt-get install -y --no-install-recommends curl ca-certificates

curl -sSL https://raw.githubusercontent.com/AdguardTeam/AdGuardHome/master/scripts/install.sh | sh -s -- -v
```

The installer sets up and starts the `AdGuardHome` service.

## Step 3 — First-run setup

Open **http://<ip>:3000**, complete the wizard (admin user + which interfaces
to listen on), and set the DNS listener to port **53** and the web UI to port
**80**. After setup the dashboard lives at **http://<ip>** (or `:80`).

## Step 4 — Point clients at it

Set your router/DHCP server's DNS option to `<ip>`, or configure clients
individually. Verify resolution and blocking:

```bash
dig @<ip> example.com +short          # should resolve
dig @<ip> doubleclick.net +short      # should be blocked (0.0.0.0 or empty)
```

## Step 5 — Backups

AdGuard state is `/opt/AdGuardHome/AdGuardHome.yaml` plus its `data/` dir. Back
it up to the `/mnt/backups` bind mount:

```bash
cat > /usr/local/bin/backup-adguard.sh << 'EOF'
#!/bin/sh
set -e
DEST=/mnt/backups; DATE=$(date +%Y%m%d_%H%M%S)
tar czf "$DEST/adguard_${DATE}.tar.gz" -C /opt/AdGuardHome AdGuardHome.yaml data
ls -dt "$DEST"/adguard_*.tar.gz | tail -n +8 | xargs -r rm -f
EOF
chmod +x /usr/local/bin/backup-adguard.sh
```

Schedule with a systemd timer (`OnCalendar=*-*-* 05:00:00`, `Persistent=true`).

## Troubleshooting

**DNS-rebind protection breaks LAN services** — AdGuard blocks answers that
resolve to private IPs by default. Services like Plex (`*.plex.direct`) need a
rebind exception, added under **Settings → DNS settings → Private reverse DNS**
/ DNS-rebind allowlist.

**Port 53 in use** — check `ss -ulnp | grep :53` inside the CT; a stray resolver
(e.g. `dnsmasq`) may need disabling.
