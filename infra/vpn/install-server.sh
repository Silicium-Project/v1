#!/usr/bin/env bash
# Met en place le VPN WireGuard du site de dev (canal dev uniquement).
# Le site dev n'est plus public : Caddy l'écoute sur 10.77.0.1:8443, joignable seulement par les pairs
# du VPN, avec le certificat de la CA interne de Caddy (tls internal). La prod n'est pas modifiée.
#
# Usage, sur le VPS, en root :  sudo SIL_DOMAIN=<domaine de prod> bash install-server.sh
# Idempotent. Sauvegarde le Caddyfile avant toute modification et ne le remplace que si Caddy le valide.
set -euo pipefail

WG_IF=wg0
WG_ADDR=10.77.0.1
WG_NET=10.77.0.0/24
WG_PORT=51820
DEV_SITE_PORT=8443
DEV_API_PORT=8081
DEV_PUBLIC_HOST=vps-910c1dbc.vps.ovh.net
CADDYFILE=/etc/caddy/Caddyfile
WG_CONF="/etc/wireguard/$WG_IF.conf"
CA_ROOT=/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt
MARK="# Managed by Silicium dev VPN"

[[ "$(id -u)" -eq 0 ]] || { echo "Lancer en root : sudo SIL_DOMAIN=... bash install-server.sh" >&2; exit 1; }
: "${SIL_DOMAIN:?SIL_DOMAIN absent (domaine de prod, nécessaire pour valider le Caddyfile)}"

if ! command -v wg >/dev/null || ! command -v wg-quick >/dev/null; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -q
  apt-get install -y -q --no-install-recommends wireguard-tools
fi

# 1. Interface serveur. La clé privée est créée ici et ne sort jamais du serveur.
install -d -m 0700 /etc/wireguard
if [[ -e "$WG_CONF" ]] && ! grep -Fqx "$MARK" "$WG_CONF"; then
  echo "Refus : configuration WireGuard non gérée déjà présente : $WG_CONF" >&2
  exit 1
fi
if [[ ! -s "$WG_CONF" ]]; then
  umask 077
  key="$(wg genkey)"
  printf '%s\n[Interface]\nAddress = %s/24\nListenPort = %s\nPrivateKey = %s\n' \
    "$MARK" "$WG_ADDR" "$WG_PORT" "$key" > "$WG_CONF"
  chmod 0600 "$WG_CONF"
fi
grep -Eq "^Address[[:space:]]*=[[:space:]]*${WG_ADDR}/" "$WG_CONF" \
  || { echo "Configuration WireGuard inattendue (adresse) : $WG_CONF" >&2; exit 1; }
wg-quick strip "$WG_IF" >/dev/null
systemctl enable --now "wg-quick@$WG_IF.service"
ip -4 addr show dev "$WG_IF" | grep -Eq "inet[[:space:]]+${WG_ADDR}/" \
  || { echo "L'interface $WG_IF n'a pas $WG_ADDR." >&2; exit 1; }

# 2. Pare-feu : WireGuard en UDP public ; le site dev seulement depuis l'interface VPN.
if ufw status 2>/dev/null | grep -q '^Status: active'; then
  ufw allow "$WG_PORT/udp" comment 'silicium dev wireguard' >/dev/null
  ufw allow in on "$WG_IF" to any port "$DEV_SITE_PORT" proto tcp comment 'silicium dev site via vpn' >/dev/null
else
  echo "ufw inactif : aucune règle ajoutée. Vérifier le pare-feu à la main." >&2
fi

# 3. Caddy : le bloc dev public est remplacé par le bloc VPN. Le reste (prod) reste tel quel.
[[ -f "$CADDYFILE" ]] || { echo "Caddyfile absent : $CADDYFILE" >&2; exit 1; }
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
backup="$CADDYFILE.bak-vpn-$stamp"
cp -p "$CADDYFILE" "$backup"
echo "sauvegarde : $backup"

work="$(mktemp)"
trap 'rm -f "$work"' EXIT
# Retire l'ancien bloc public du dev et tout bloc VPN précédent, puis écrit le bloc VPN (lié à $WG_ADDR).
awk -v host="$DEV_PUBLIC_HOST" -v vpn="https://$WG_ADDR:$DEV_SITE_PORT {" '
  $0 == host " {" || $0 == vpn { skip = 1; next }
  /^# Site DEV/ { next }
  skip { if ($0 == "}") skip = 0; next }
  { print }
' "$CADDYFILE" > "$work"

cat >> "$work" <<CADDY

# Site DEV : joignable seulement depuis le VPN WireGuard ($WG_NET), certificat de la CA interne de Caddy.
https://$WG_ADDR:$DEV_SITE_PORT {
	bind $WG_ADDR
	tls internal
	encode zstd gzip
	reverse_proxy 127.0.0.1:$DEV_API_PORT
	header {
		Strict-Transport-Security "max-age=31536000"
		-Server
	}
}
CADDY

if ! SIL_DOMAIN="$SIL_DOMAIN" caddy validate --config "$work" --adapter caddyfile >/dev/null 2>&1; then
  echo "Caddyfile proposé invalide : rien n'est changé (voir $work)." >&2
  trap - EXIT
  exit 1
fi
install -o root -g root -m 0644 "$work" "$CADDYFILE"
# Redémarrage (pas rechargement) : un changement d'adresse d'écoute ne se recharge pas à chaud, et le
# port 8443 est déjà tenu par l'ancienne configuration. Coupure brève de la prod, le temps du redémarrage.
if ! systemctl restart caddy; then
  echo "Redémarrage Caddy refusé : restauration de $backup" >&2
  cp -p "$backup" "$CADDYFILE"
  systemctl restart caddy || true
  exit 1
fi

# 4. Vérifications et données à transmettre (publiques : empreinte de la CA, clé publique du serveur).
ss -tlnp | grep -q "$WG_ADDR:$DEV_SITE_PORT" || { echo "Caddy n'écoute pas sur $WG_ADDR:$DEV_SITE_PORT" >&2; exit 1; }
echo "Site dev : https://$WG_ADDR:$DEV_SITE_PORT (VPN uniquement)"
echo "Clé publique du serveur : $(wg show "$WG_IF" public-key)"
echo "Endpoint : $DEV_PUBLIC_HOST:$WG_PORT"
echo "Ajouter un pair : sudo bash infra/vpn/manage-peer.sh add <nom> <clé publique>"
if [[ -s "$CA_ROOT" ]]; then
  echo "Empreinte SHA-256 de la CA dev (à vérifier hors bande avant installation) :"
  openssl x509 -in "$CA_ROOT" -noout -fingerprint -sha256
else
  echo "CA interne pas encore générée : ouvrir une fois https://$WG_ADDR:$DEV_SITE_PORT via le VPN puis relancer." >&2
fi
echo "VPN_DEV_OK"
