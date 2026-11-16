#!/usr/bin/env bash
# Gère les pairs du VPN du site dev. La clé privée d'un client ne quitte jamais son poste : le client
# génère sa paire de clés, et seule la clé publique arrive sur le serveur.
#
# Usage, sur le VPS, en root :
#   sudo bash infra/vpn/manage-peer.sh add <nom> <clé publique du client>
#   sudo bash infra/vpn/manage-peer.sh revoke <nom>
#   sudo bash infra/vpn/manage-peer.sh list
set -euo pipefail

WG_IF=wg0
WG_CONF="/etc/wireguard/$WG_IF.conf"
WG_HOST_PREFIX=10.77.0
WG_PORT=51820
DEV_PUBLIC_HOST=vps-910c1dbc.vps.ovh.net

usage() {
  echo "Usage : manage-peer.sh add <nom> <clé publique> | revoke <nom> | list" >&2
  exit 2
}

[[ "$(id -u)" -eq 0 ]] || { echo "Lancer en root." >&2; exit 1; }
[[ -s "$WG_CONF" ]] || { echo "Interface absente : lancer d'abord install-server.sh" >&2; exit 1; }

action="${1:-}"
name="${2:-}"

peer_exists() { grep -Fqx "# peer: $1" "$WG_CONF"; }

backup_conf() {
  cp -p "$WG_CONF" "$WG_CONF.bak-peer-$(date -u +%Y%m%dT%H%M%SZ)"
}

case "$action" in
  add)
    key="${3:-}"
    [[ "$name" =~ ^[a-z0-9][a-z0-9-]{0,31}$ ]] || { echo "Nom invalide (minuscules, chiffres, tirets)." >&2; exit 2; }
    [[ "$key" =~ ^[A-Za-z0-9+/]{43}=$ ]] || { echo "Clé publique WireGuard invalide." >&2; exit 2; }
    peer_exists "$name" && { echo "Pair déjà présent : $name" >&2; exit 1; }
    grep -Fq "PublicKey = $key" "$WG_CONF" && { echo "Cette clé publique est déjà utilisée." >&2; exit 1; }
    # Première adresse libre dans 10.77.0.2 .. 10.77.0.254 (10.77.0.1 est le serveur).
    ip_addr=""
    for last in $(seq 2 254); do
      if ! grep -Fq "AllowedIPs = $WG_HOST_PREFIX.$last/32" "$WG_CONF"; then
        ip_addr="$WG_HOST_PREFIX.$last"
        break
      fi
    done
    [[ -n "$ip_addr" ]] || { echo "Plus d'adresse libre dans le VPN." >&2; exit 1; }
    backup_conf
    printf '\n# peer: %s\n[Peer]\nPublicKey = %s\nAllowedIPs = %s/32\n' "$name" "$key" "$ip_addr" >> "$WG_CONF"
    wg set "$WG_IF" peer "$key" allowed-ips "$ip_addr/32"
    server_key="$(wg show "$WG_IF" public-key)"
    echo "Pair ajouté : $name -> $ip_addr"
    echo ""
    echo "Configuration à compléter sur le poste (la clé privée reste sur le poste) :"
    echo "[Interface]"
    echo "PrivateKey = <clé privée du client>"
    echo "Address = $ip_addr/32"
    echo ""
    echo "[Peer]"
    echo "PublicKey = $server_key"
    echo "Endpoint = $DEV_PUBLIC_HOST:$WG_PORT"
    echo "AllowedIPs = $WG_HOST_PREFIX.1/32"
    echo "PersistentKeepalive = 25"
    ;;
  revoke)
    peer_exists "$name" || { echo "Pair inconnu : $name" >&2; exit 1; }
    key="$(awk -v tag="# peer: $name" '
      $0 == tag { found = 1; next }
      found && /^PublicKey = / { print $3; exit }
    ' "$WG_CONF")"
    [[ -n "$key" ]] || { echo "Clé introuvable pour $name" >&2; exit 1; }
    backup_conf
    tmp="$(mktemp)"
    trap 'rm -f "$tmp"' EXIT
    awk -v tag="# peer: $name" '
      $0 == tag { skip = 1; next }
      skip && $0 == "" { skip = 0; next }
      skip { next }
      { print }
    ' "$WG_CONF" > "$tmp"
    install -o root -g root -m 0600 "$tmp" "$WG_CONF"
    wg set "$WG_IF" peer "$key" remove
    echo "Pair révoqué : $name"
    ;;
  list)
    awk '
      /^# peer: / { name = substr($0, 9) }
      /^AllowedIPs = / && name != "" { print name, $3; name = "" }
    ' "$WG_CONF"
    ;;
  *)
    usage
    ;;
esac
