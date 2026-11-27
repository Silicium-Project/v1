#!/usr/bin/env bash
# Construit les paquets Linux de l'application de bureau (.deb, .rpm et AppImage) avec app-linux.json
# (nom, somme SHA-256, taille).
#   prod : dépôt apps/web/public/downloads/, paquet « silicium » (tauri.conf.json).
#   dev  : dist/linux-dev/ (jamais le site de production), paquet « silicium-dev » (tauri.dev.conf.json).
#
# Usage, depuis la racine du dépôt, dans WSL ou sous Linux :
#   bash infra/release/build-linux-desktop.sh          # prod
#   bash infra/release/build-linux-desktop.sh dev      # dev
#
# Prérequis (une fois) : node 22+ et npm dans le PATH, cargo (rustup), rsync, python3, rpmbuild, et les
# paquets de développement Tauri (libwebkit2gtk-4.1-dev, libayatana-appindicator3-dev, librsvg2-dev,
# patchelf). Les binaires Linux doivent être dans dist/linux-amd64/ (voir docs/technical/deployment.md).
# rpmbuild sans installation système : exporter RPM_CONFIGDIR vers le dossier des macros rpm.
#
# Le build se fait dans une copie du dépôt sur le système de fichiers Linux ($SIL_BUILD_DIR), pour
# que node_modules soit celui de Linux et non celui de Windows.
set -euo pipefail

CHANNEL="${1:-prod}"
case "$CHANNEL" in
  prod) CONF="src-tauri/tauri.conf.json"; PKG="silicium"; OUT_REL="apps/web/public/downloads" ;;
  dev) CONF="src-tauri/tauri.dev.conf.json"; PKG="silicium-dev"; OUT_REL="dist/linux-dev" ;;
  *) echo "canal inconnu : $CHANNEL (prod ou dev)" >&2; exit 1 ;;
esac

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WORK="${SIL_BUILD_DIR:-$HOME/silicium-build}"
TARGET="${CARGO_TARGET_DIR:-$HOME/silicium-target-desktop-linux}"
VERSION="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$ROOT/apps/desktop/src-tauri/tauri.conf.json")"
DOWNLOADS="$ROOT/$OUT_REL"

for name in node npm cargo rsync python3 rpmbuild rpm; do
  command -v "$name" >/dev/null || { echo "absent du PATH : $name" >&2; exit 1; }
done
for file in silicium-node silicium-daemon sil-raytracer-block.wasm sil-pdf-block.wasm libonnxruntime.so; do
  test -f "$ROOT/dist/linux-amd64/$file" || { echo "absent : dist/linux-amd64/$file" >&2; exit 1; }
done

echo "== copie de travail : $WORK ($CHANNEL, $PKG)"
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
(cd "$WORK/apps/desktop" && CARGO_TARGET_DIR="$TARGET" npm run build -- --config "$CONF")

BUNDLE="$TARGET/release/bundle"
# Le nom de fichier suit productName (« Silicium » ou « Silicium Dev ») : on prend le seul paquet de chaque sorte.
shopt -s nullglob
DEBS=("$BUNDLE"/deb/*.deb)
RPMS=("$BUNDLE"/rpm/*.rpm)
APPS=("$BUNDLE"/appimage/*.AppImage)
shopt -u nullglob
[ "${#DEBS[@]}" -eq 1 ] && [ "${#RPMS[@]}" -eq 1 ] && [ "${#APPS[@]}" -eq 1 ] \
  || { echo "paquets inattendus dans $BUNDLE (attendu : un .deb, un .rpm, un AppImage)" >&2; exit 1; }
DEB_SRC="${DEBS[0]}"
RPM_SRC="${RPMS[0]}"
APP_SRC="${APPS[0]}"

# Le nom du paquet doit être celui du canal : un .deb « silicium » en dev écraserait la prod sur un poste.
# Comparaison insensible à la casse et aux espaces (« Silicium Dev » → « silicium-dev »).
normalize() { tr '[:upper:] ' '[:lower:]-'; }
DEB_NAME="$(dpkg-deb -f "$DEB_SRC" Package)"
RPM_NAME="$(rpm -qp --queryformat '%{NAME}' "$RPM_SRC")"
[ "$(printf %s "$DEB_NAME" | normalize)" = "$PKG" ] \
  || { echo "paquet .deb inattendu : « $DEB_NAME », attendu $PKG" >&2; exit 1; }
[ "$(printf %s "$RPM_NAME" | normalize)" = "$PKG" ] \
  || { echo "paquet .rpm inattendu : « $RPM_NAME », attendu $PKG" >&2; exit 1; }

# Le moteur de rendu et le module pdf sont des WebAssembly posés dans lib/Silicium (node_process.rs,
# node_registry.rs les y cherchent) ; le nœud et le runtime ONNX sont dans le paquet.
CONTENT=("silicium-node" "silicium-daemon" "sil-raytracer-block.wasm" "sil-pdf-block.wasm" "libonnxruntime.so")
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

echo "== publication dans $OUT_REL"
mkdir -p "$DOWNLOADS"
DEB="$(basename "$DEB_SRC")"
APP="$(basename "$APP_SRC")"
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
