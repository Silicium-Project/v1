#!/usr/bin/env bash
# Copie les binaires compilés et les unités systemd sur le VPS, puis recharge systemd.
# Usage, depuis la racine du dépôt : infra/vps/deploy.sh utilisateur@hôte
# Les binaires viennent de dist/linux-amd64/ (compilés localement). Rien n'est compilé sur le VPS.
set -euo pipefail

host="${1:?usage : infra/vps/deploy.sh utilisateur@hôte}"
root="$(cd "$(dirname "$0")/../.." && pwd)"

# Règle : aucun déploiement sans sauvegarde réussie des données (journal, node.seed, base API).
# Ce script ne déplace, ne renomme et ne supprime jamais le journal ni les données du nœud.
# Un changement de format de journal se fait par une procédure séparée, décidée explicitement.
"$root/infra/vps/backup.sh" "$host" || { echo "sauvegarde échouée : déploiement annulé" >&2; exit 1; }
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

for file in silicium-api silicium-node sil-sandbox-launch sil-raytracer-block libonnxruntime.so; do
	test -f "$root/dist/linux-amd64/$file" || { echo "absent : dist/linux-amd64/$file" >&2; exit 1; }
	cp "$root/dist/linux-amd64/$file" "$stage/"
done
cp "$root/infra/systemd/silicium-api.service" "$root/infra/systemd/silicium-node.service" "$stage/"

# Clé et known_hosts explicites : sous WSL, ~/.ssh n'est pas celui de Windows (aucune clé trouvée,
# « lost connection »). Variables : SIL_SSH_KEY, SIL_SSH_KNOWN_HOSTS. Sur Windows, utiliser deploy.ps1.
key="${SIL_SSH_KEY:-$HOME/.ssh/id_ed25519}"
known="${SIL_SSH_KNOWN_HOSTS:-$HOME/.ssh/known_hosts}"
test -f "$key" || { echo "clé SSH absente : $key (SIL_SSH_KEY)" >&2; exit 1; }
# BatchMode : jamais de question interactive (confirmation d'hôte, mot de passe), sinon le déploiement reste bloqué.
ssh_opts=(-o BatchMode=yes -o ConnectTimeout=15 -o StrictHostKeyChecking=yes -i "$key" -o "UserKnownHostsFile=$known")
scp -q "${ssh_opts[@]}" "$stage/silicium-api" "$stage/silicium-node" "$stage/sil-sandbox-launch" "$stage/sil-raytracer-block" \
	"$stage/libonnxruntime.so" "$stage/silicium-api.service" "$stage/silicium-node.service" "$host:/tmp/"

# Écriture atomique : un binaire en cours d'exécution ne peut pas être écrasé sur place.
ssh "${ssh_opts[@]}" "$host" 'set -e
for bin in silicium-api silicium-node sil-sandbox-launch sil-raytracer-block libonnxruntime.so; do
	sudo install -m 0755 "/tmp/$bin" "/opt/silicium/bin/$bin.new"
	sudo mv -f "/opt/silicium/bin/$bin.new" "/opt/silicium/bin/$bin"
	rm -f "/tmp/$bin"
done
for unit in silicium-api silicium-node; do
	sudo install -m 0644 "/tmp/$unit.service" "/etc/systemd/system/$unit.service"
	rm -f "/tmp/$unit.service"
done
sudo systemctl daemon-reload
sudo systemctl try-restart silicium-node.service silicium-api.service
echo DEPLOY_OK'
