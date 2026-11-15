#!/usr/bin/env bash
# Construit les paquets Linux de l'application de bureau (.deb, .rpm et AppImage) et les dépose dans
# apps/web/public/downloads/, avec app-linux.json (nom, somme SHA-256, taille).
#
# Usage, depuis la racine du dépôt, dans WSL ou sous Linux :
#   bash infra/release/build-linux-desktop.sh
#
# Prérequis (une fois) : node 22+ et npm dans le PATH, cargo (rustup), rsync, python3, rpmbuild, et les
# paquets de développement Tauri (libwebkit2gtk-4.1-dev, libayatana-appindicator3-dev, librsvg2-dev,
# patchelf). Les binaires Linux doivent être dans dist/linux-amd64/ (voir docs/technical/deployment.md).
# rpmbuild sans installation système : exporter RPM_CONFIGDIR vers le dossier des macros rpm.
#
# Le build se fait dans une copie du dépôt sur le système de fichiers Linux ($SIL_BUILD_DIR), pour
# que node_modules soit celui de Linux et non celui de Windows.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WORK="${SIL_BUILD_DIR:-$HOME/silicium-build}"
TARGET="${CARGO_TARGET_DIR:-$HOME/silicium-target-desktop-linux}"
VERSION="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$ROOT/apps/desktop/src-tauri/tauri.conf.json")"
DOWNLOADS="$ROOT/apps/web/public/downloads"

for name in node npm cargo rsync python3 rpmbuild rpm; do
  command -v "$name" >/dev/null || { echo "absent du PATH : $name" >&2; exit 1; }
done
for file in silicium-node sil-raytracer-block sil-sandbox-launch libonnxruntime.so; do
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

echo "== build Tauri (deb, rpm et AppImage) : $TARGET"
# Les sorties précédentes sont retirées : sinon un ancien paquet d'une autre version serait pris.
rm -rf "$TARGET/release/bundle"
(cd "$WORK/apps/desktop" && CARGO_TARGET_DIR="$TARGET" npm run build)

BUNDLE="$TARGET/release/bundle"
DEB_SRC="$BUNDLE/deb/Silicium_${VERSION}_amd64.deb"
RPM_SRC="$BUNDLE/rpm/Silicium-${VERSION}-1.x86_64.rpm"
APP_SRC="$BUNDLE/appimage/Silicium_${VERSION}_amd64.AppImage"
for file in "$DEB_SRC" "$RPM_SRC" "$APP_SRC"; do
  test -f "$file" || { echo "paquet absent après build : $file" >&2; exit 1; }
done

# Le lanceur d'isolation doit être dans le paquet : le nœud le cherche à côté de lui (cli.rs).
CONTENT=("silicium-node" "sil-raytracer-block" "sil-sandbox-launch" "libonnxruntime.so")
echo "== contrôle du contenu du .deb"
LISTING="$(mktemp)"
trap 'rm -f "$LISTING"' EXIT
dpkg-deb -c "$DEB_SRC" > "$LISTING"
for name in "${CONTENT[@]}"; do
  grep -q "/$name\$" "$LISTING" || { echo "absent du .deb : $name" >&2; exit 1; }
done
echo "== contrôle du contenu du .rpm"
rpm -qpl "$RPM_SRC" > "$LISTING"
for name in "${CONTENT[@]}"; do
  grep -q "/$name\$" "$LISTING" || { echo "absent du .rpm : $name" >&2; exit 1; }
done

echo "== publication dans apps/web/public/downloads"
mkdir -p "$DOWNLOADS"
DEB="Silicium_${VERSION}_amd64.deb"
APP="Silicium_${VERSION}_amd64.AppImage"
RPM="$(basename "$RPM_SRC")"
cp "$DEB_SRC" "$DOWNLOADS/$DEB"
cp "$APP_SRC" "$DOWNLOADS/$APP"
cp "$RPM_SRC" "$DOWNLOADS/$RPM"
python3 - "$DOWNLOADS" "$VERSION" "$APP" "$DEB" "$RPM" <<'PY'
import hashlib, json, os, sys
folder, version, app, deb, rpm = sys.argv[1:6]
def entry(kind, name):
    path = os.path.join(folder, name)
    with open(path, "rb") as handle:
        digest = hashlib.sha256(handle.read()).hexdigest()
    return {"kind": kind, "file": name, "sha256": digest, "bytes": os.path.getsize(path)}
out = {"version": version, "packages": [entry("AppImage", app), entry("deb", deb), entry("rpm", rpm)]}
with open(os.path.join(folder, "app-linux.json"), "w", encoding="utf-8") as handle:
    json.dump(out, handle, indent=2, ensure_ascii=False)
    handle.write("\n")
print(json.dumps(out, indent=2))
PY
echo "== terminé : $DOWNLOADS"
