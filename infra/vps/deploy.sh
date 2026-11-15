#!/usr/bin/env bash
# Copie les binaires compilés et les unités systemd sur le VPS, puis recharge systemd.
# Usage, depuis la racine du dépôt : SIL_ENV=prod|dev infra/vps/deploy.sh utilisateur@hôte
# SIL_ENV vaut prod par défaut. dev écrit uniquement dans /opt/silicium-dev, /etc/silicium-dev,
# /var/lib/silicium-dev et les services *-dev : jamais dans les chemins ou services de prod.
# Les binaires viennent de dist/linux-amd64/ (compilés localement). Rien n'est compilé sur le VPS.
#
# Ce script ne sauvegarde pas : la sauvegarde se fait à part, depuis le poste (infra/vps/backup.sh),
# avant d'approuver un déploiement de production.
# Il ne déplace, ne renomme et ne supprime jamais le journal ni les données du nœud.
set -euo pipefail

host="${1:?usage : SIL_ENV=prod|dev infra/vps/deploy.sh utilisateur@hôte}"
root="$(cd "$(dirname "$0")/../.." && pwd)"

case "${SIL_ENV:-prod}" in
  prod) suffix="" ;;
  dev) suffix="-dev" ;;
  *) echo "SIL_ENV inconnu : ${SIL_ENV} (prod ou dev)" >&2; exit 1 ;;
esac
opt="/opt/silicium$suffix"
etc="/etc/silicium$suffix"
var="/var/lib/silicium$suffix"

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

for file in silicium-api silicium-node sil-sandbox-launch sil-raytracer-block libonnxruntime.so; do
	test -f "$root/dist/linux-amd64/$file" || { echo "absent : dist/linux-amd64/$file" >&2; exit 1; }
	cp "$root/dist/linux-amd64/$file" "$stage/"
done

# Unités : celles du dépôt, réécrites pour l'environnement (chemins et noms des services).
for unit in silicium-api silicium-node; do
	sed -e "s#/opt/silicium/#$opt/#g; s#/etc/silicium/#$etc/#g; s#/var/lib/silicium/#$var/#g" \
		"$root/infra/systemd/$unit.service" > "$stage/$unit$suffix.service"
done

# Clé et known_hosts explicites : sous WSL, ~/.ssh n'est pas celui de Windows (aucune clé trouvée,
# « lost connection »). Variables : SIL_SSH_KEY, SIL_SSH_KNOWN_HOSTS. Sur Windows, utiliser deploy.ps1.
key="${SIL_SSH_KEY:-$HOME/.ssh/id_ed25519}"
known="${SIL_SSH_KNOWN_HOSTS:-$HOME/.ssh/known_hosts}"
test -f "$key" || { echo "clé SSH absente : $key (SIL_SSH_KEY)" >&2; exit 1; }
# BatchMode : jamais de question interactive (confirmation d'hôte, mot de passe), sinon le déploiement reste bloqué.
ssh_opts=(-o BatchMode=yes -o ConnectTimeout=15 -o StrictHostKeyChecking=yes -i "$key" -o "UserKnownHostsFile=$known")
scp -q "${ssh_opts[@]}" "$stage/silicium-api" "$stage/silicium-node" "$stage/sil-sandbox-launch" "$stage/sil-raytracer-block" \
	"$stage/libonnxruntime.so" "$stage/silicium-api$suffix.service" "$stage/silicium-node$suffix.service" "$host:/tmp/"

# Écriture atomique : un binaire en cours d'exécution ne peut pas être écrasé sur place.
ssh "${ssh_opts[@]}" "$host" "set -e
sudo install -d -m 0755 '$opt/bin'
for bin in silicium-api silicium-node sil-sandbox-launch sil-raytracer-block libonnxruntime.so; do
	sudo install -m 0755 \"/tmp/\$bin\" '$opt/bin/'\"\$bin\"'.new'
	sudo mv -f '$opt/bin/'\"\$bin\"'.new' '$opt/bin/'\"\$bin\"
	rm -f \"/tmp/\$bin\"
done
for unit in silicium-api$suffix silicium-node$suffix; do
	sudo install -m 0644 \"/tmp/\$unit.service\" \"/etc/systemd/system/\$unit.service\"
	rm -f \"/tmp/\$unit.service\"
done
sudo systemctl daemon-reload
sudo systemctl try-restart silicium-node$suffix.service silicium-api$suffix.service
echo DEPLOY_OK $suffix"
