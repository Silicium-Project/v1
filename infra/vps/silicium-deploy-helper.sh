#!/usr/bin/env bash
# Helper root du déploiement, installé en /usr/local/sbin/silicium-deploy par deploy-user.sh.
# C'est le seul point privilégié des comptes de déploiement. sudoers n'autorise qu'une forme :
#   silicium-deploy-<env> ALL=(root) NOPASSWD: /usr/local/sbin/silicium-deploy <env> *
# Les contenus arrivent sur stdin (binaires, unités, archives) : le helper ne lit jamais un chemin
# fourni par l'appelant, donc pas de lien symbolique à suivre. Chaque nom est validé par liste blanche.
#
# Usage :
#   silicium-deploy <prod|dev> bin <nom>            < binaire
#   silicium-deploy <prod|dev> unit <silicium-api|silicium-node>  < fichier .service
#   silicium-deploy <prod|dev> restart
#   silicium-deploy <prod|dev> web                  < archive tar du site
#   silicium-deploy <prod|dev> repos                < archive tar des dépôts (relative à download/)
#   silicium-deploy <prod|dev> repos-get            > archive tar des paquets existants
#   silicium-deploy <prod|dev> probe <chemin>       contrôle HTTPS du site de l'environnement
set -euo pipefail

usage() {
  echo "usage : silicium-deploy <prod|dev> bin|unit|restart|web|repos|repos-get|probe [argument]" >&2
  exit 2
}

[[ "$(id -u)" -eq 0 ]] || { echo "silicium-deploy : lancer par sudo." >&2; exit 1; }

env_name="${1:-}"
action="${2:-}"
arg="${3:-}"

case "$env_name" in
  prod) sfx=""; base="https://siliciumlabs.org"; ca="" ;;
  dev) sfx="-dev"; base="https://10.77.0.1:8443"
       ca=/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt ;;
  *) usage ;;
esac
opt="/opt/silicium$sfx"
web="$opt/web"
download="$web/download"

case "$action" in
  bin)
    case "$arg" in
      silicium-api|silicium-node|sil-sandbox-launch|sil-raytracer-block|libonnxruntime.so) ;;
      *) echo "binaire refusé : $arg" >&2; exit 2 ;;
    esac
    install -d -m 0755 "$opt/bin"
    tmp="$opt/bin/.$arg.new"
    cat > "$tmp"
    chmod 0755 "$tmp"
    mv -f "$tmp" "$opt/bin/$arg"
    echo "ok binaire $env_name $arg"
    ;;
  unit)
    case "$arg" in
      silicium-api|silicium-node) ;;
      *) echo "unité refusée : $arg" >&2; exit 2 ;;
    esac
    tmp="$(mktemp /etc/systemd/system/.silicium-unit.XXXXXX)"
    cat > "$tmp"
    # Refus de ce qui permettrait une exécution en root : préfixes + ! @ - sur les lignes Exec*,
    # utilisateur autre que silicium, binaire hors de $opt/bin.
    if grep -Eq '^Exec[A-Za-z]*=[+!@-]' "$tmp" \
      || grep -Fq '..' "$tmp" \
      || ! grep -Eq '^User=silicium$' "$tmp" \
      || grep -E '^ExecStart=' "$tmp" | grep -Fqv "ExecStart=$opt/bin/"; then
      rm -f "$tmp"
      echo "unité refusée (contrôle de contenu) : $arg" >&2
      exit 2
    fi
    chmod 0644 "$tmp"
    mv -f "$tmp" "/etc/systemd/system/$arg$sfx.service"
    echo "ok unité $env_name $arg$sfx.service"
    ;;
  restart)
    systemctl daemon-reload
    systemctl try-restart "silicium-node$sfx.service" "silicium-api$sfx.service"
    echo "ok redémarrage $env_name"
    ;;
  web)
    install -d -m 0755 "$web"
    tar -C "$web" --no-same-owner -xf -
    echo "ok site $env_name"
    ;;
  repos)
    install -d -m 0755 "$download"
    tar -C "$download" --no-same-owner -xf -
    echo "ok dépôts $env_name (rien supprimé)"
    ;;
  repos-get)
    install -d -m 0755 "$download"
    tar -C "$download" -cf - --ignore-failed-read dnf/Packages apt/pool
    ;;
  probe)
    [[ "$arg" =~ ^/[A-Za-z0-9._/-]*$ ]] || { echo "chemin refusé : $arg" >&2; exit 2; }
    if [[ -n "$ca" ]]; then
      curl -fsS --max-time 20 --cacert "$ca" "$base$arg" > /dev/null
    else
      curl -fsS --max-time 20 "$base$arg" > /dev/null
    fi
    echo "ok $base$arg"
    ;;
  *)
    usage
    ;;
esac
