#!/usr/bin/env bash
# Creates and configures a adguard LXC on Proxmox VE. Run on the host as root.
set -euo pipefail
VMID="${1:?Usage: $0 <vmid> [options]}"
# TODO: pct create / config / install adguard. Mirror jellyfin/lxc/provision.sh.
echo "[provision] adguard LXC $VMID — not yet implemented"
