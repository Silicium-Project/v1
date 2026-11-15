#!/usr/bin/env bash
set -euo pipefail

# Pare-feu : tout fermé sauf SSH, HTTP(S) et P2P (TCP et QUIC, port 4001).
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp comment 'ssh'
sudo ufw allow 80/tcp comment 'http'
sudo ufw allow 443/tcp comment 'https'
sudo ufw allow 4001/tcp comment 'silicium p2p tcp'
sudo ufw allow 4001/udp comment 'silicium p2p quic'
sudo ufw --force enable

# SSH : clés seulement. La connexion par clé a été vérifiée avant ce changement.
sudo tee /etc/ssh/sshd_config.d/10-silicium.conf >/dev/null <<'CONF'
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
CONF
sudo sshd -t
sudo systemctl reload ssh

# Mises à jour : paquets et correctifs de sécurité automatiques.
export DEBIAN_FRONTEND=noninteractive
sudo apt-get update -q
sudo apt-get -y -q -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold upgrade
sudo apt-get install -y -q caddy unattended-upgrades
sudo systemctl enable --now unattended-upgrades
# Caddy est configuré plus tard (domaine requis) : pas de serveur par défaut en écoute.
sudo systemctl disable --now caddy

# Utilisateur système sans shell, et dossiers dédiés.
id silicium >/dev/null 2>&1 || sudo useradd --system --home-dir /var/lib/silicium --shell /usr/sbin/nologin silicium
sudo install -d -m 0755 /opt/silicium/bin /opt/silicium/web /etc/silicium
sudo install -d -o silicium -g silicium -m 0750 /var/lib/silicium /var/lib/silicium/node /var/lib/silicium/api

echo "== ufw"
sudo ufw status verbose
echo "== sshd"
sudo sshd -T | grep -Ei '^(passwordauthentication|permitrootlogin) '
echo "== reboot requis ?"
if [ -f /var/run/reboot-required ]; then echo oui; else echo non; fi
echo SOCLE_OK
