#!/usr/bin/env bash
# Sauvegarde des données du VPS Silicium (journal, identité du nœud, base de l'API, configuration).
#
# Lecture seule : rien n'est écrit sur le VPS. Les archives sont tirées en flux vers ce poste, dans
# un dossier horodaté, avec un manifeste SHA-256. Le script échoue si une donnée attendue manque.
#
# Usage, depuis la racine du dépôt :
#   infra/vps/backup.sh utilisateur@hôte [dossier de destination]
# Clé SSH : SIL_SSH_KEY (défaut ~/.ssh/id_ed25519). Destination par défaut : ~/silicium-backups.
#
# Appelé automatiquement par infra/vps/deploy.sh avant toute modification : pas de déploiement sans
# sauvegarde réussie.
set -euo pipefail

host="${1:?usage : infra/vps/backup.sh utilisateur@hôte [dossier]}"
root="${2:-$HOME/silicium-backups}"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
dest="$root/vps-$stamp"
key="${SIL_SSH_KEY:-$HOME/.ssh/id_ed25519}"
ssh_opts=(-o BatchMode=yes -o ConnectTimeout=15 -o StrictHostKeyChecking=yes -i "$key")

umask 077
mkdir -p "$dest"

# Données de nœud et de l'API ; le rendu (render-work), le cache de contenus et les embeddings sont
# régénérables et exclus. Configuration du nœud et de l'API séparée (contient des secrets).
ssh "${ssh_opts[@]}" "$host" 'sudo tar -C /var/lib/silicium --exclude=node/render-work --exclude=node/content --exclude=node/embed -czf - node api' > "$dest/donnees.tgz"
ssh "${ssh_opts[@]}" "$host" 'sudo tar -C /etc -czf - silicium' > "$dest/configuration.tgz"

tar -tzf "$dest/donnees.tgz" > "$dest/contenu-donnees.txt"
tar -tzf "$dest/configuration.tgz" > "$dest/contenu-configuration.txt"

# Ce qui doit absolument être présent : identité du nœud, journal, base de l'API.
for must in '^node/node\.seed$' '^node/ledger/' '^api/silicium\.db$'; do
  grep -q "$must" "$dest/contenu-donnees.txt" || { echo "absent de la sauvegarde : $must" >&2; exit 1; }
done
grep -q '^silicium/node\.toml$' "$dest/contenu-configuration.txt" || { echo "absent : /etc/silicium/node.toml" >&2; exit 1; }

(cd "$dest" && sha256sum donnees.tgz configuration.tgz > SHA256SUMS)
(cd "$dest" && sha256sum -c SHA256SUMS >/dev/null)
echo "sauvegarde vérifiée : $dest"
