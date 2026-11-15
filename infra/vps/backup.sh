#!/usr/bin/env bash
# Sauvegarde des données du VPS Silicium (journal, identité du nœud, base de l'API, configuration).
#
# Lecture seule côté VPS : rien n'est écrit sur le serveur. Les archives sont tirées en flux vers ce
# poste, vérifiées, puis chiffrées avec une clé publique GPG. Seul le détenteur de la clé privée peut les lire.
#
# Usage, depuis la racine du dépôt, sur le poste (pas dans la CI) :
#   SIL_BACKUP_RECIPIENT=<empreinte GPG> infra/vps/backup.sh utilisateur@hôte [dossier de destination]
# Variables : SIL_SSH_KEY (défaut ~/.ssh/id_ed25519), SIL_BACKUP_RECIPIENT (obligatoire, empreinte de la clé
# publique GPG), destination par défaut ~/silicium-backups.
#
# Ce script n'est appelé par aucun déploiement. Avant d'approuver un déploiement de production, lancer
# une sauvegarde depuis le poste.
set -euo pipefail

host="${1:?usage : infra/vps/backup.sh utilisateur@hôte [dossier]}"
root="${2:-$HOME/silicium-backups}"
recipient="${SIL_BACKUP_RECIPIENT:?SIL_BACKUP_RECIPIENT absent : empreinte de la clé publique GPG}"
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
rm -f "$dest/contenu-donnees.txt" "$dest/contenu-configuration.txt"

# Chiffrement : seule la clé publique est nécessaire ici. Les versions en clair sont supprimées ensuite.
for f in donnees.tgz configuration.tgz; do
  gpg --batch --yes --trust-model always --recipient "$recipient" --encrypt --output "$dest/$f.gpg" "$dest/$f"
  rm -f "$dest/$f"
done

(cd "$dest" && sha256sum donnees.tgz.gpg configuration.tgz.gpg > SHA256SUMS)
(cd "$dest" && sha256sum -c SHA256SUMS >/dev/null)
echo "sauvegarde chiffrée et vérifiée : $dest"
