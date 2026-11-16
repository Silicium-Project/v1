#!/usr/bin/env bash
# Installe l'entretien hebdomadaire du VPS (housekeeping.sh) comme service systemd root avec son timer.
# À lancer une fois, en root, depuis la racine du dépôt : sudo bash infra/vps/install-housekeeping.sh
set -euo pipefail

[[ "$(id -u)" -eq 0 ]] || { echo "Lancer en root." >&2; exit 1; }
src="$(cd "$(dirname "$0")" && pwd)/housekeeping.sh"
[[ -f "$src" ]] || { echo "absent : $src" >&2; exit 1; }

install -o root -g root -m 0750 "$src" /usr/local/sbin/silicium-housekeeping

cat > /etc/systemd/system/silicium-housekeeping.service <<'UNIT'
[Unit]
Description=Silicium : entretien du VPS (restes de déploiement, caches, journaux tournés)
Documentation=https://github.com/Silicium-Project/v1/blob/main/infra/vps/housekeeping.sh

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/silicium-housekeeping --apply
Nice=10
IOSchedulingClass=idle
UNIT

cat > /etc/systemd/system/silicium-housekeeping.timer <<'TIMER'
[Unit]
Description=Silicium : entretien hebdomadaire du VPS

[Timer]
OnCalendar=Sun *-*-* 04:30:00
RandomizedDelaySec=30min
Persistent=true

[Install]
WantedBy=timers.target
TIMER

systemctl daemon-reload
systemctl enable --now silicium-housekeeping.timer
echo "entretien installé : prochain passage"
systemctl list-timers silicium-housekeeping.timer --no-pager | head -3
