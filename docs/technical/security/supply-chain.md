# Chaîne d'approvisionnement : ce qui est épinglé, ce qui ne l'est pas

Ce document liste ce qui entre dans un nœud ou un paquet, d'où ça vient, ce qui le vérifie, et les
écarts connus. Les règles de sandbox et de clés sont ailleurs (`sandbox.md`, `keys.md`).

## Runtime ONNX Runtime 1.31.1

- **Source** : wheel PyPI `onnxruntime==1.31.1`, versions `cp312` (`win_amd64` et
  `manylinux_2_28_x86_64`).
- **Épinglage** : `tools/runtime/fetch_onnxruntime.py` contient le SHA-256 du wheel et celui de la
  bibliothèque extraite. Le script refuse tout fichier qui ne correspond pas, et n'écrit rien dans le
  dossier cible.
  - Windows : wheel `ae8a9a35…6130`, `onnxruntime.dll` `e45f61e5…0306`.
  - Linux : wheel `1f3dea47…d536`, `libonnxruntime.so.1.31.1` `fc44112f…d1c2d3`.
- **Chargement** : le nœud charge la bibliothèque depuis le dossier de son binaire (ou
  `../lib/Silicium` sous Linux), par chemin, sans recherche dans le `PATH`. Qui peut écrire dans ce
  dossier peut remplacer la bibliothèque : même confiance que le binaire lui-même.
- **Écart** : `infra/vps/deploy.sh` copie `libonnxruntime.so` sans vérifier son hash à l'arrivée sur le
  VPS. Le hash a été vérifié à la main le 11 novembre.

## Modèle et tokenizer d'embedding

- **Source** : Hugging Face, `intfloat/multilingual-e5-small` (fp32 `model.onnx` et `tokenizer.json`).
- **Épinglage** : SHA-256 du modèle `ca456c06…8665` et du tokenizer `0b44a9d7…4c39`, dans
  `crates/sil-wl-embed/src/lib.rs` et le descripteur `contracts/workloads/embedding-e5-small.json`.
- **Vérification** : `sil_model::ensure` télécharge dans `<dest>.part`, vérifie le hash, puis renomme.
  Un fichier qui ne correspond pas n'atteint jamais le dossier du modèle.
- **Conséquence** : changer de modèle exige un nouveau build (les hash sont dans le code et dans le
  descripteur, donc dans la concordance Rust, Go et TS).
- **Écart** : le téléchargement se fait en HTTPS, sans épinglage du certificat. La confiance repose
  sur Hugging Face et l'autorité de certification ; le hash limite le dégât à un téléchargement
  refusé, pas à un modèle accepté.

## Paquets publiés (site)

- **Archives du nœud** (`silicium-node-linux-amd64.tar.gz`, `silicium-node-windows-x86_64.zip`) : fichier
  `.sha256` à côté de chaque archive, et `manifest.json`. Ces sommes sont servies par le même site, sur
  le même canal : elles protègent contre une corruption, pas contre un site compromis.
- **Installeur Windows** : signé Authenticode par `beforeBundleCommand` (`scripts/sign.mjs`). La
  signature protège l'utilisateur contre une modification après publication. Le certificat est sur le
  poste de développement, pas en CI.
- **Paquets Linux** (`.deb`, AppImage) : non signés. Leur somme est dans `app-linux.json`.
- **Écart** : aucune signature détachée (GPG ou équivalente) sur les paquets et les sommes. C'est la
  phase 9 du plan.

## Extraction de texte PDF (site, navigateur)

- **Source** : npm `pdfjs-dist` 6.4.299 (Mozilla pdf.js, Apache-2.0), version exacte dans `apps/web/package.json`
  et `package-lock.json` (résolution figée, `npm ci` en CI).
- **Où ça tourne** : dans le navigateur de l'auteur de la demande, jamais sur un nœud. Le PDF ne quitte
  pas l'appareil ; seul le texte extrait est envoyé, comme un fichier texte.
- **Écart** : un PDF malveillant peut faire échouer le moteur dans l'onglet de l'auteur seulement. Le
  worker est un fichier du site (`script-src 'self'` de l'application bureau le permet).

## Dépendances Rust et Go

- `Cargo.lock` et `services/api/go.sum` committés ; CI en `--locked`. L'API Go a une seule dépendance
  externe : `modernc.org/sqlite` (base SQLite, Go pur), avec ses dépendances transitives.
- `cargo deny check` et `cargo audit` dans `ci.yml` (job `checks`), exécutés à chaque push.
- `ort` épinglé en `2.0.0-rc.2`, `ort-sys` épinglé en `=2.0.0-rc.2` : une résolution plus récente a
  cassé la compilation (voir l'historique du dépôt). Ne pas relâcher ce pin sans mesure de
  déterminisme (`embedding-workload.md`, §9).

## CI et déploiement

- **Actions** : `actions/checkout@v4`, `actions/setup-go@v5`, `actions/upload-artifact@v4`,
  `actions/download-artifact@v4`, épinglées par tag majeur, pas par SHA. Le job `deliver` n'utilise
  aucune action tierce pour Rust : `rustup` est fourni par le runner (`dtolnay/rust-toolchain@stable`
  a été retiré pour cette raison).
- **Secrets de déploiement** : prévus dans l'environnement GitHub `production` (`DEPLOY_SSH_KEY`) et
  `development`, avec l'hôte et `known_hosts` en variables. Non configurés à ce jour.
- **Déploiement manuel** (aujourd'hui) : clé SSH du poste de développement (`~/.ssh/id_ed25519`), hôte
  épinglé dans `known_hosts`, `ssh` et `scp` en `BatchMode=yes` (sans cela, une confirmation d'hôte
  bloque le déploiement sans rien signaler).
- **Écart** : le VPS reçoit des binaires non signés, copiés depuis le poste de développement (phase 9).

## Points à traiter

1. Vérifier le SHA-256 de `libonnxruntime.so` dans `deploy.sh` après copie sur le VPS.
2. Épingler les actions GitHub par SHA de commit, avec Dependabot.
3. Signer les paquets Linux et les sommes publiées.
4. Choisir la stratégie de certificat pour l'installeur en CI.
