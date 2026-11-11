#!/usr/bin/env bash
# Construit les paquets Linux de l'application de bureau (.deb et AppImage) et les dépose dans
# apps/web/public/downloads/, avec app-linux.json (nom, somme SHA-256, taille).
#
# Usage, depuis la racine du dépôt, dans WSL ou sous Linux :
#   bash infra/release/build-linux-desktop.sh
#
# Prérequis (une fois) : node 22+ et npm dans le PATH, cargo (rustup), rsync, python3, et les
# paquets de développement Tauri (libwebkit2gtk-4.1-dev, libayatana-appindicator3-dev, librsvg2-dev,
# patchelf). Les binaires Linux doivent être dans dist/linux-amd64/ (voir docs/technical/deployment.md).
#
# Le build se fait dans une copie du dépôt sur le système de fichiers Linux ($SIL_BUILD_DIR), pour
# que node_modules soit celui de Linux et non celui de Windows.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WORK="${SIL_BUILD_DIR:-$HOME/silicium-build}"
TARGET="${CARGO_TARGET_DIR:-$HOME/silicium-target-desktop-linux}"
VERSION="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$ROOT/apps/desktop/src-tauri/tauri.conf.json")"
DOWNLOADS="$ROOT/apps/web/public/downloads"

for name in node npm cargo rsync python3; do
  command -v "$name" >/dev/null || { echo "absent du PATH : $name" >&2; exit 1; }
done
for file in silicium-node sil-raytracer-block libonnxruntime.so; do
  test -f "$ROOT/dist/linux-amd64/$file" || { echo "absent : dist/linux-amd64/$file" >&2; exit 1; }
done

echo "== copie de travail : $WORK"
mkdir -p "$WORK"
rsync -a --delete \
  --exclude node_modules --exclude target --exclude .git \
  --exclude /dist/windows-amd64 --exclude /dist/bundle-staging --exclude /apps/web/dist \
  --exclude /apps/web/dist-desktop --exclude /apps/web/public/downloads \
  "$ROOT/" "$WORK/"

echo "== dépendances web et bureau (Linux)"
(cd "$WORK/apps/web" && npm ci)
(cd "$WORK/apps/desktop" && npm ci)

echo "== build Tauri (deb et AppImage) : $TARGET"
(cd "$WORK/apps/desktop" && CARGO_TARGET_DIR="$TARGET" npm run build)

BUNDLE="$TARGET/release/bundle"
DEB_SRC="$(ls "$BUNDLE"/deb/*.deb | head -n 1)"
APP_SRC="$(ls "$BUNDLE"/appimage/*.AppImage | head -n 1)"

echo "== contrôle du contenu du .deb"
LISTING="$(mktemp)"
trap 'rm -f "$LISTING"' EXIT
dpkg-deb -c "$DEB_SRC" > "$LISTING"
# Le paquet de bureau ne contient pas sil-sandbox-launch (il est dans l'archive du nœud).
for name in silicium-node sil-raytracer-block libonnxruntime.so; do
  grep -q "/$name\$" "$LISTING" || { echo "absent du .deb : $name" >&2; exit 1; }
done

echo "== publication dans apps/web/public/downloads"
mkdir -p "$DOWNLOADS"
DEB="Silicium_${VERSION}_amd64.deb"
APP="Silicium_${VERSION}_amd64.AppImage"
cp "$DEB_SRC" "$DOWNLOADS/$DEB"
cp "$APP_SRC" "$DOWNLOADS/$APP"
python3 - "$DOWNLOADS" "$VERSION" "$APP" "$DEB" <<'PY'
import hashlib, json, os, sys
folder, version, app, deb = sys.argv[1:5]
def entry(kind, name):
    path = os.path.join(folder, name)
    with open(path, "rb") as handle:
        digest = hashlib.sha256(handle.read()).hexdigest()
    return {"kind": kind, "file": name, "sha256": digest, "bytes": os.path.getsize(path)}
out = {"version": version, "packages": [entry("AppImage", app), entry("deb", deb)]}
with open(os.path.join(folder, "app-linux.json"), "w", encoding="utf-8") as handle:
    json.dump(out, handle, indent=2, ensure_ascii=False)
    handle.write("\n")
print(json.dumps(out, indent=2))
PY
echo "== terminé : $DOWNLOADS"
