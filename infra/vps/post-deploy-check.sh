#!/usr/bin/env bash
# Checks an environment right after a deployment: services active, local API health, binary hashes
# matching the artifacts that were built, and public URLs answering. Fails on the first problem.
# Usage: SIL_ENV=prod|dev post-deploy-check.sh <user@host> <dossier des binaires locaux>
set -euo pipefail

host="${1:?usage : post-deploy-check.sh user@host dossier-binaires}"
local_bin="${2:?dossier des binaires locaux manquant}"
key="${SIL_SSH_KEY:?SIL_SSH_KEY absent}"
known="${SIL_SSH_KNOWN_HOSTS:?SIL_SSH_KNOWN_HOSTS absent}"
ssh_opts=(-o BatchMode=yes -o ConnectTimeout=15 -o StrictHostKeyChecking=yes -i "$key" -o "UserKnownHostsFile=$known")

case "${SIL_ENV:-prod}" in
  prod) suffix=""; api_port=8080; base="https://siliciumlabs.org" ;;
  # dev : site joignable seulement par le VPN (10.77.0.1:8443, CA interne de Caddy). Le contrôle se fait
  # donc depuis le serveur, qui est sur le VPN ; le runner de CI ne peut pas l'atteindre.
  dev) suffix="-dev"; api_port=8081; base="https://10.77.0.1:8443" ;;
  *) echo "SIL_ENV inconnu : ${SIL_ENV}" >&2; exit 1 ;;
esac
opt="/opt/silicium$suffix"

# 1. Services active on the server.
for unit in silicium-node$suffix silicium-api$suffix; do
  state="$(ssh "${ssh_opts[@]}" "$host" "systemctl is-active $unit.service" || true)"
  [ "$state" = active ] || { echo "service non actif : $unit ($state)" >&2; exit 1; }
  echo "ok service $unit"
done

# 2. Installed binaries have the hashes of the artifacts that were built.
for bin in silicium-node silicium-api; do
  expected="$(sha256sum "$local_bin/$bin" | cut -d' ' -f1)"
  actual="$(ssh "${ssh_opts[@]}" "$host" "sha256sum $opt/bin/$bin | cut -d' ' -f1")"
  [ "$expected" = "$actual" ] || { echo "hash différent : $bin" >&2; exit 1; }
  echo "ok binaire $bin"
done

# 3. Local API answers on the server, and the site answers: publicly for prod, from the server over
# the VPN for dev (the helper probes with the CA that Caddy generated; the deploy account cannot read it).
ssh "${ssh_opts[@]}" "$host" "curl -fsS --max-time 10 http://127.0.0.1:$api_port/health > /dev/null"
echo "ok api locale"
if [ "$SIL_ENV" = dev ]; then
  ssh "${ssh_opts[@]}" "$host" "sudo /usr/local/sbin/silicium-deploy dev probe /"
  echo "ok site dev via VPN $base/"
  ssh "${ssh_opts[@]}" "$host" "sudo /usr/local/sbin/silicium-deploy dev probe /download/apt/dists/stable/InRelease"
  echo "ok dépôt APT dev via VPN"
else
  curl -fsS --max-time 20 -o /dev/null "$base/"
  echo "ok site public $base/"
  curl -fsS --max-time 20 -o /dev/null "$base/download/apt/dists/stable/InRelease"
  echo "ok dépôt APT public"
fi
echo "POST_DEPLOY_OK $SIL_ENV"
