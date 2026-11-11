# Tests : commandes, tests ignorés, état

Ce document dit quoi lancer, ce qui est couvert, et ce qui ne l'est pas. Un test absent ou ignoré
n'est pas une réussite : les tests `#[ignore]` ci-dessous ne tournent pas dans `cargo test` seul.

## Commandes de base (depuis la racine du dépôt)

```bash
cargo test --workspace --locked                    # Rust : unitaires et intégration
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
cd services/api && go test -race ./...              # Go
cd apps/web && npm run typecheck && npm test        # TypeScript : vue-tsc, vitest (i18n, contraste, axe)
node tools/workloads/generate.mjs --check           # descripteurs : concordance Rust / Go / TS
python3 tools/check_size.py .                       # règle des 300 lignes par fichier
```

Sous Windows, `python3` peut ne pas exister : utiliser `python`.

## Tests ignorés (demandent des fichiers externes)

Ces tests téléchargent ou chargent ONNX Runtime 1.31.1 et le modèle. Ils se lancent avec `--ignored` et
trois variables d'environnement :

| Variable | Fichier |
|---|---|
| `SIL_EMBED_RUNTIME` | `onnxruntime.dll` ou `libonnxruntime.so` (1.31.1, voir `tools/runtime/fetch_onnxruntime.py`) |
| `SIL_EMBED_MODEL` | `model.onnx` (multilingual-e5-small fp32, SHA-256 `ca456c06…8665`) |
| `SIL_EMBED_TOKENIZER` | `tokenizer.json` (SHA-256 `0b44a9d7…4c39`) |

```bash
SIL_EMBED_RUNTIME=... SIL_EMBED_MODEL=... SIL_EMBED_TOKENIZER=... \
  cargo test -p sil-wl-embed --test reference -- --ignored --nocapture   # vecteurs mesurés, à 1e-5 près
SIL_EMBED_RUNTIME=... SIL_EMBED_MODEL=... SIL_EMBED_TOKENIZER=... \
  cargo test -p sil-wl-embed -- --ignored                                # honnête passe, falsifié échoue
cargo test -p sil-node embedding -- --ignored                            # e2e : worker, vérificateur, finalisation, paiement
```

Résultats constatés (novembre 2026) :
- `reference` : ok en 4,2 s avec la DLL 1.31.1 (Windows).
- `embedding` (e2e multi-nœuds, en mémoire) : ok en environ 10 s.
- `sil-wl-embed` (honnête et falsifié) : à relancer après chaque changement du moteur ; non relancé
  pour ce document.

Le WARN `ort` (« expected GetVersionString to return '1.17.x' ») apparaît au chargement de la
bibliothèque. Il n'indique pas un échec : `ort` avertit quand la version runtime est plus récente que
celle attendue (`ort-2.0.0-rc.2/src/lib.rs` : `Ordering::Greater` → `warn`, `Less` → erreur). Le test
de référence, qui passe avec 1.31.1, confirme que la sortie est correcte.

## Ce qui est couvert

- **Descripteurs** : `contracts/workloads/*.json` génère les fichiers Rust, Go et TS ; `--check`
  refuse une dérive.
- **Tolérance** : `crates/sil-attest` et `crates/sil-ledger/tests/tolerance.rs` couvrent le mode du
  descripteur, le seuil de cosinus, le rejeu d'une attestation exacte. La graine (tirage de
  l'échantillon) n'a pas de test dédié.
- **Lots** : `crates/sil-batch` (15 tests) et `crates/sil-wl-image` (18 tests) : format, options,
  vérification par image.
- **Embedding côté site** : `embedding.test.ts`, `EmbeddingInput.test.ts`, `documents.test.ts` (ZIP, TXT,
  formats refusés, limites), `pdf.test.ts` (lignes par page). L'extraction PDF réelle (pdf.js et son
  worker) ne tourne pas sous happy-dom : elle a été vérifiée à la main dans le navigateur.
- **Site** : contraste WCAG (`palette.ts`), i18n (parité fr/en), axe-core sur les pages de connexion,
  confidentialité et 404 ; `embedding.ts` (comptage, refus d'un texte vide ou trop gros) ;
  `EmbeddingInput.test.ts`.
- **Desktop** : `apps/desktop/src-tauri` (clippy, tests).
- **API** : `services/api` (politique, télémétrie, workloads).

## Ce qui n'est pas couvert

- **Réseau réel** : aucun test avec plusieurs machines réelles. Les tests `sil-node` tournent en
  mémoire. Le simulateur (`infra/sim`, image `silicium-node:sim`) n'est pas dans la CI.
- **Wallet réel** : la signature Phantom n'est pas testée (voir `web-and-desktop.md`, limites).
- **AVX-512** : non mesuré (voir `embedding-workload.md`, §9).
- **Installeur** : l'installation silencieuse est un test manuel (`/S /D=<dossier>`), pas un test CI.
  Procédure : vérifier que `onnxruntime.dll` (ou `libonnxruntime.so` sous Linux) est à côté de
  `silicium-node`, puis comparer son SHA-256 à celui du wheel 1.31.1.
- **Déploiement VPS** : vérifié à la main (hash et journal, voir `deployment.md`).
- **Accessibilité** : lecteur d'écran et clavier complet non testés (voir `web-and-desktop.md`).

## État de la CI (novembre 2026)

`.github/workflows/ci.yml` sur `main` échoue sur trois jobs, au 11 novembre :

- `checks` : règle des 300 lignes. 13 fichiers dépassent, dont `apps/web/src/i18n/fr-site.ts` (402),
  `apps/web/src/i18n/en.ts` (363), `crates/sil-attest/src/tolerance.rs` (339),
  `crates/sil-node/src/verifier_loop.rs` (359), `crates/sil-node/src/cli.rs` (305).
- `go-api` : `gofmt -l` liste `internal/config/config.go`, `internal/server/handlers_network.go`,
  `internal/server/handlers_network_status.go`. Correction : `gofmt -w services/api`.
- `desktop` : `cargo fmt --check` échoue sur `apps/desktop/src-tauri`. Correction :
  `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.

`.github/workflows/deliver.yml` se déclenche sur `main` et `dev`. Son déploiement échoue faute
d'environnements et de secrets (voir `deployment.md`).

Un job `CodeQL` passe. Les jobs `ci` sur les branches `refonte-v1-relais` échouaient déjà.
