#!/usr/bin/env bash
# Copie les binaires compilés et les unités systemd sur le VPS, puis recharge systemd.
# Usage, depuis la racine du dépôt : SIL_ENV=prod|dev infra/vps/deploy.sh utilisateur@hôte
# SIL_ENV vaut prod par défaut. dev écrit uniquement dans /opt/silicium-dev, /etc/silicium-dev,
# /var/lib/silicium-dev et les services *-dev : jamais dans les chemins ou services de prod.
# Les binaires viennent de dist/linux-amd64/ (compilés localement). Rien n'est compilé sur le VPS.
#
# Le compte de déploiement n'a pas de droits root généraux : les fichiers passent sur stdin au helper
# /usr/local/sbin/silicium-deploy (infra/vps/silicium-deploy-helper.sh), qui valide chaque nom et écrit
# atomiquement. Le compte ne peut agir que sur SIL_ENV (sudoers : « silicium-deploy <env> * »).
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
env_name="${SIL_ENV:-prod}"
helper="sudo /usr/local/sbin/silicium-deploy $env_name"

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

for file in silicium-api silicium-node sil-sandbox-launch sil-raytracer-block libonnxruntime.so; do
	test -f "$root/dist/linux-amd64/$file" || { echo "absent : dist/linux-amd64/$file" >&2; exit 1; }
	cp "$root/dist/linux-amd64/$file" "$stage/"
done

# Unités : celles du dépôt, réécrites pour l'environnement (chemins et noms des services).
for unit in silicium-api silicium-node; do
	sed -e "s#/opt/silicium/#/opt/silicium$suffix/#g; s#/etc/silicium/#/etc/silicium$suffix/#g; s#/var/lib/silicium/#/var/lib/silicium$suffix/#g" \
		"$root/infra/systemd/$unit.service" > "$stage/$unit$suffix.service"
done

# Clé et known_hosts explicites : sous WSL, ~/.ssh n'est pas celui de Windows (aucune clé trouvée,
# « lost connection »). Variables : SIL_SSH_KEY, SIL_SSH_KNOWN_HOSTS. Sur Windows, utiliser deploy.ps1.
key="${SIL_SSH_KEY:-$HOME/.ssh/id_ed25519}"
known="${SIL_SSH_KNOWN_HOSTS:-$HOME/.ssh/known_hosts}"
test -f "$key" || { echo "clé SSH absente : $key (SIL_SSH_KEY)" >&2; exit 1; }
# BatchMode : jamais de question interactive (confirmation d'hôte, mot de passe), sinon le déploiement reste bloqué.
ssh_opts=(-o BatchMode=yes -o ConnectTimeout=15 -o StrictHostKeyChecking=yes -i "$key" -o "UserKnownHostsFile=$known")

# Binaires : un par appel, contenu sur stdin. Le helper écrit à côté puis renomme (pas d'écrasement sur place).
for bin in silicium-api silicium-node sil-sandbox-launch sil-raytracer-block libonnxruntime.so; do
	ssh "${ssh_opts[@]}" "$host" "$helper bin $bin" < "$stage/$bin"
done

# Unités : contrôlées par le helper (pas de préfixe root, User=silicium, binaire dans $opt/bin).
for unit in silicium-api silicium-node; do
	ssh "${ssh_opts[@]}" "$host" "$helper unit $unit" < "$stage/$unit$suffix.service"
done

ssh "${ssh_opts[@]}" "$host" "$helper restart"
echo "DEPLOY_OK $suffix"
