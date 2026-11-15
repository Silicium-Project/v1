#!/usr/bin/env bash
# Writes the update manifest for one release: version, channel, and the SHA-256 and size of every
# artifact, then signs it with the release key. The client refuses an unsigned or altered manifest.
# Usage: RELEASE_KEY_FPR=<empreinte> [GPG_PASSPHRASE=...] publish-update-manifest.sh <version> <canal> <out-dir> <dossier-artefacts>...
set -euo pipefail

version="${1:?version manquante}"
channel="${2:?canal manquant (dev|prod)}"
out="${3:?dossier de sortie manquant}"
shift 3
fpr="${RELEASE_KEY_FPR:?RELEASE_KEY_FPR absent}"
mkdir -p "$out"

files=()
for dir in "$@"; do
  while IFS= read -r -d '' f; do files+=("$f"); done < <(find "$dir" -maxdepth 1 -type f \( -name '*.AppImage' -o -name '*.deb' -o -name '*.rpm' -o -name '*.exe' -o -name '*.zip' -o -name '*.tar.gz' \) -print0)
done
(( ${#files[@]} > 0 )) || { echo "aucun artefact" >&2; exit 1; }

{
  printf '{\n  "version": "%s",\n  "channel": "%s",\n  "generated_at": "%s",\n  "files": [\n' "$version" "$channel" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  first=1
  for f in "${files[@]}"; do
    [ $first -eq 1 ] || printf ',\n'
    first=0
    printf '    {"name": "%s", "bytes": %s, "sha256": "%s"}' "$(basename "$f")" "$(stat -c %s "$f")" "$(sha256sum "$f" | cut -d' ' -f1)"
  done
  printf '\n  ]\n}\n'
} > "$out/manifest.json"

gpg --batch --yes --pinentry-mode loopback ${GPG_PASSPHRASE:+--passphrase "$GPG_PASSPHRASE"} \
  --local-user "$fpr" --detach-sign --armor -o "$out/manifest.json.asc" "$out/manifest.json"
gpg --batch --verify "$out/manifest.json.asc" "$out/manifest.json"
echo "manifeste : $out/manifest.json (${#files[@]} artefacts)"
