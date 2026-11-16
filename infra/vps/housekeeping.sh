#!/usr/bin/env bash
# Entretien automatique du VPS : espace disque et restes d'anciens déploiements. Lancé chaque semaine par
# le timer silicium-housekeeping.timer (infra/systemd). Par défaut : simulation, rien n'est supprimé.
#
# Usage, en root : bash housekeeping.sh [--apply]
#
# Jamais supprimé, quoi qu'il arrive : données du nœud et de l'API (/var/lib/silicium*, ledger, node.seed,
# silicium.db), dépôts publiés (web/download*, web/downloads*), configuration (/etc), sauvegardes, journaux
# du nœud. Seuls sont visés : les restes de déploiement, les caches, les journaux système tournés et les
# fichiers de transit du déploiement.
set -euo pipefail

apply=0
[[ "${1:-}" == "--apply" ]] && apply=1
[[ "$(id -u)" -eq 0 ]] || { echo "Lancer en root." >&2; exit 1; }

say() { echo "$*"; logger -t silicium-housekeeping -- "$*" || true; }
run() {
  if (( apply )); then "$@"; else echo "simulation : $*"; fi
}
say "housekeeping : $( (( apply )) && echo application || echo simulation )"

# 1. Paquets téléchargés (cache apt) et journal systemd limité à 14 jours / 200 Mo.
run apt-get clean
run journalctl --vacuum-time=14d --vacuum-size=200M

# 2. Journaux système déjà tournés (logrotate garde les plus récents). wtmp, btmp et lastlog ne sont pas visés.
find /var/log -maxdepth 2 -type f \( -name '*.gz' -o -name '*.[0-9]' \) -mtime +14 -print0 2>/dev/null \
  | while IFS= read -r -d '' file; do run rm -f -- "$file"; done

# 3. Restes d'un déploiement interrompu : binaires en cours de copie, archives partielles.
for root in /opt/silicium /opt/silicium-dev; do
  find "$root/bin" -maxdepth 1 -type f \( -name '.*.new' -o -name '*.before-*' \) -mtime +1 -print0 2>/dev/null \
    | while IFS= read -r -d '' file; do run rm -f -- "$file"; done
done
for home in /home/silicium-deploy-dev /home/silicium-deploy-prod; do
  [[ -d "$home/deploy-stage" ]] && find "$home/deploy-stage" -mindepth 1 -mtime +1 -print0 \
    | while IFS= read -r -d '' file; do run rm -rf -- "$file"; done
done
find /tmp -maxdepth 1 -name 'silicium-deploy-*' -mtime +1 -print0 2>/dev/null \
  | while IFS= read -r -d '' dir; do run rm -rf -- "$dir"; done

# 4. Caches de l'utilisateur d'administration (régénérés automatiquement).
run find /home/ubuntu/.cache -mindepth 1 -maxdepth 1 -mtime +30 -exec rm -rf -- {} +

say "housekeeping : terminé"
