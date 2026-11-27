#!/usr/bin/env bash
# Publishes the DNF and APT repositories of the environment: keeps the packages already on the server,
# adds the new ones, regenerates the metadata, signs it with the release key, and uploads the result.
#
# Usage (CI, after the packages are downloaded): SIL_ENV=prod|dev RELEASE_KEY_FPR=<fingerprint> \
#   GPG_PASSPHRASE=<optional> infra/release/publish-repos.sh <user@host> <dossier des paquets>
#
# The release key must already be imported in the GPG keyring (the workflow does it).
# Nothing is deleted on the server: old packages stay in the repositories.
set -euo pipefail

host="${1:?usage : publish-repos.sh user@host dossier-des-paquets}"
packages="${2:?dossier des paquets manquant}"
fpr="${RELEASE_KEY_FPR:?RELEASE_KEY_FPR absent (empreinte de la clé de release)}"

case "${SIL_ENV:-prod}" in
  prod) remote_root="/opt/silicium/web/download" ;;
  dev) remote_root="/opt/silicium-dev/web/download" ;;
  *) echo "SIL_ENV inconnu : ${SIL_ENV}" >&2; exit 1 ;;
esac

key="${SIL_SSH_KEY:?SIL_SSH_KEY absent}"
known="${SIL_SSH_KNOWN_HOSTS:?SIL_SSH_KNOWN_HOSTS absent}"
ssh_opts=(-o BatchMode=yes -o ConnectTimeout=15 -o StrictHostKeyChecking=yes -i "$key" -o "UserKnownHostsFile=$known")

work="$(mktemp -d)"
# Le dossier de travail est envoyé tel quel (tar) puis extrait en root sur le serveur : son mode doit
# être 755. mktemp le crée en 700, et la racine web download/ devenait root:700 : l'API ne lisait plus les dépôts.
chmod 755 "$work"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/dnf/Packages" "$work/apt/pool/main/s" "$work/apt/dists/stable/main/binary-amd64"

# 1. Current repositories from the server (read-only, through the deploy helper: the account has no
# general read access to the web root's packages otherwise).
ssh "${ssh_opts[@]}" "$host" "sudo /usr/local/sbin/silicium-deploy $SIL_ENV repos-get" | tar -C "$work" -xf -

# 2. New packages.
shopt -s nullglob
rpms=("$packages"/*.rpm); debs=("$packages"/*.deb)
(( ${#rpms[@]} + ${#debs[@]} > 0 )) || { echo "aucun paquet dans $packages" >&2; exit 1; }
if (( ${#rpms[@]} )); then cp -f "${rpms[@]}" "$work/dnf/Packages/"; fi
# Le pool est rangé par nom de paquet : « silicium » (prod) et « silicium-dev » (dev) ont chacun le leur.
for deb in "${debs[@]}"; do
  name="$(dpkg-deb -f "$deb" Package)"
  mkdir -p "$work/apt/pool/main/s/$name"
  cp -f "$deb" "$work/apt/pool/main/s/$name/"
done

# 3. DNF metadata, signed (repomd.xml.asc, as the existing repository).
createrepo_c --quiet "$work/dnf"
gpg --batch --yes --pinentry-mode loopback ${GPG_PASSPHRASE:+--passphrase "$GPG_PASSPHRASE"} \
  --local-user "$fpr" --detach-sign --armor -o "$work/dnf/repodata/repomd.xml.asc" "$work/dnf/repodata/repomd.xml"

# 4. APT indexes, signed (Release.gpg and InRelease).
(
  cd "$work/apt"
  apt-ftparchive packages pool > dists/stable/main/binary-amd64/Packages
  gzip -kf dists/stable/main/binary-amd64/Packages
  apt-ftparchive release dists/stable > dists/stable/Release
  gpg --batch --yes --pinentry-mode loopback ${GPG_PASSPHRASE:+--passphrase "$GPG_PASSPHRASE"} \
    --local-user "$fpr" --detach-sign --armor -o dists/stable/Release.gpg dists/stable/Release
  gpg --batch --yes --pinentry-mode loopback ${GPG_PASSPHRASE:+--passphrase "$GPG_PASSPHRASE"} \
    --local-user "$fpr" --clearsign -o dists/stable/InRelease dists/stable/Release
)

# 5. Public key served next to the repositories (replaces the previous one).
gpg --batch --armor --export "$fpr" > "$work/silicium-release.asc"

# 6. Checks before upload: signatures verify with the public key only.
gpg --batch --verify "$work/dnf/repodata/repomd.xml.asc" "$work/dnf/repodata/repomd.xml"
gpg --batch --verify "$work/apt/dists/stable/Release.gpg" "$work/apt/dists/stable/Release"

# 7. Upload through the helper (it extracts without deleting anything on the server).
if [ -n "${UPDATE_DIR:-}" ]; then
  mkdir -p "$work/update"
  cp -a "$UPDATE_DIR/." "$work/update/"
fi
tar -C "$work" -cf - . | ssh "${ssh_opts[@]}" "$host" "sudo /usr/local/sbin/silicium-deploy $SIL_ENV repos"
echo "dépôts publiés ($SIL_ENV) : $remote_root"
