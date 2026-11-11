#!/usr/bin/env bash
# Copie les binaires compilés et les unités systemd sur le VPS, puis recharge systemd.
# Usage, depuis la racine du dépôt : infra/vps/deploy.sh utilisateur@hôte
# Les binaires viennent de dist/linux-amd64/ (compilés localement). Rien n'est compilé sur le VPS.
set -euo pipefail

host="${1:?usage : infra/vps/deploy.sh utilisateur@hôte}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

for file in silicium-api silicium-node sil-sandbox-launch sil-raytracer-block libonnxruntime.so; do
	test -f "$root/dist/linux-amd64/$file" || { echo "absent : dist/linux-amd64/$file" >&2; exit 1; }
	cp "$root/dist/linux-amd64/$file" "$stage/"
done
cp "$root/infra/systemd/silicium-api.service" "$root/infra/systemd/silicium-node.service" "$stage/"

scp -q "$stage/silicium-api" "$stage/silicium-node" "$stage/sil-sandbox-launch" "$stage/sil-raytracer-block" \
	"$stage/libonnxruntime.so" "$stage/silicium-api.service" "$stage/silicium-node.service" "$host:/tmp/"

# Écriture atomique : un binaire en cours d'exécution ne peut pas être écrasé sur place.
ssh "$host" 'set -e
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
