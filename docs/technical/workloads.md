# Workloads : descripteurs et procédure

Un workload (type de calcul) se décrit **une fois**, dans `contracts/workloads/<nom>.json`. Trois
consommateurs lisent ce descripteur, générés par `tools/workloads/generate.mjs` :

| Consommateur | Fichier généré | Ce qu'il en tire |
|---|---|---|
| Nœud et chaîne (Rust) | `crates/sil-workload-spec/src/generated.rs` | unité, limites, politique de vérification, découpage |
| API (Go) | `services/api/internal/workloadspec/generated.go` | découpage, tarifs : `policy.For` |
| Site (TypeScript) | `apps/web/src/workloads/generated.ts` | liste des workloads, champs du formulaire, dépôt |

Les fichiers générés ne se modifient pas à la main. Le Go et le TypeScript ont un test de concordance
avec les descripteurs (`workloadspec_test.go`, `workloads.test.ts`). Le Rust n'en a pas : sa
concordance se vérifie par `node tools/workloads/generate.mjs --check`.

## Ce qui est générique, ce qui reste du code

- **Générique (descripteur)** : identifiant, unité, champs demandés à l'utilisateur, taille max
  de l'entrée, découpage (`max_fragments`, `target_size`), tarifs, politique de vérification
  (`recompute`, `sampling` ou `sampling_tolerance`), limites d'exécution, possibilité de dépôt sur le site.
- **Code (par workload)** : le découpage exact de l'entrée, l'exécution d'un fragment et la
  vérification d'un résultat. Un descripteur ne peut pas décrire un calcul.
- **Code (par type d'unité)** : `sil-chain-core/src/units.rs`, `escrow.ts` (`maxPayout`). Un type
  connu (`bytes_input`, `pixels_band`) ne demande aucune modification on-chain.

## Champs d'un descripteur

```json
{
  "schema": "silicium.workload/v1",
  "id": "rendu/raytracer",
  "description": "Texte affiché dans la documentation générée.",
  "unit": "pixels_band",
  "label_key": "submit.workloadRender",
  "input": { "params": ["width", "height"], "max_side": 4096, "max_input_bytes": 8388608 },
  "geometry": { "block_rows": 8 },
  "split": { "max_fragments": 16, "target_size": 32 },
  "terms": { "reward_per_unit": 1, "verifier_reward_per_unit": 1 },
  "verification": { "policy": "sampling", "min_cheat_fraction": 0.2, "target_detection": 0.99 },
  "limits": { "wall_ms": 600000, "memory_bytes": 1073741824, "output_bytes": 268435456 },
  "deposit": "unsupported"
}
```

- `unit` : `bytes_input` (une unité = un octet de l'entrée) ou `pixels_band` (une unité = un pixel
  d'une bande de blocs de lignes). `pixels_band` exige `params` = `["width", "height"]` et
  `geometry.block_rows`, `input.max_side`.
- `params` : seuls `width` et `height` sont connus de la spec du job. Ils sont envoyés en en-tête
  de l'entrée, en u32 little-endian, dans l'ordre déclaré.
- `split.target_size` : taille visée d'un fragment, dans l'unité de découpage (octets pour
  `bytes_input`, blocs pour `pixels_band`).
- `deposit` : `supported` seulement si le site sait calculer le dépôt (unité `bytes_input`).
- `verification` : `sampling_tolerance` ajoute `cos_min` (seuil de cosinus), `model_sha256` (hash du
  modèle épinglé) et `ort_version` (version ONNX Runtime épinglée). Le nœud refuse un workload dont
  le modèle ou la version ne correspondent pas. Voir `embedding-workload.md`.

## Ajouter un workload

1. Écrire `contracts/workloads/<nom>.json` (champs ci-dessus).
2. `node tools/workloads/generate.mjs` : régénère les trois fichiers et valide le descripteur.
3. Créer le crate `crates/sil-wl-<nom>` : implémenter `Workload` (`split`, `execute`,
   `verify`, `combine`). Pour `id`, `limits` et `policy`, appeler `limits_of` et `policy_of` avec le
   descripteur (`sil_workload_spec::<NOM>`).
4. Enregistrer le workload dans `registry_for` (`crates/sil-node/src/cli.rs`).
5. Si le type d'unité est nouveau : ajouter la règle dans `sil-chain-core/src/units.rs`, la
   redéployer sur le programme, puis ajouter le calcul de dépôt dans `apps/web/src/chain/escrow.ts`.
6. Ajouter les clés de libellé (`label_key`) dans `fr.ts` et `en.ts`.
7. Lancer les tests : `cargo test --workspace`, `go test ./...` dans `services/api`,
   `npx vitest run` dans `apps/web`, et `node tools/workloads/generate.mjs --check`.

## Lots de fichiers : `image/batch`

Le workload prend un lot dans le conteneur `SILBATCH` (crate `sil-batch`) : un en-tête
(type de média, format de sortie, taille cible, nombre d'enregistrements) puis les fichiers. Les
paramètres vivent dans le lot, pas dans la spec : `params` reste vide.

- **`image/batch`** (`sil-wl-image`, Rust pur) : redimensionnement et réencodage PNG ou JPEG. Un
  fragment est une plage d'images. Chaque image est un bloc vérifiable : le résultat engage le hash
  de chaque image encodée, et la politique `sampling` fait refaire seulement les images tirées. Un
  vérificateur contrôle d'abord la cohérence des hash, sans rendu.
- Un fragment par demande, même s'il y a moins d'enregistrements : les fragments en trop ont une
  plage vide et un résultat vide. Le découpage en octets du réseau (`bytes_input`) ne suit pas le
  découpage en enregistrements : le paiement reste proportionnel aux octets, mais les fragments vides
  sont payés au prorata de leur part d'octets du conteneur.

- Téléchargement : le résultat d'un lot est une archive ZIP, un fichier par entrée (`0001.png`,
  `0002.jpg`… selon les premiers octets). Le nœud la donne par `GET /v1/jobs/{id}/archive` (octets
  bruts, 64 Mo max) ; l'API la convertit en ZIP (`jobs/batch.go`). `GET /v1/jobs/{id}/outputs` reste
  limité à 4 Ko par fragment (JSON hexadécimal) : il ne sert plus aux lots.


### Options du lot (version 2 de SILBATCH)

Zone de 16 octets après le nombre d'enregistrements (`sil-batch/src/options.rs`) : qualité JPEG, mode de redimensionnement, rotation et indicateurs (niveaux de gris, miroirs). Les octets 4 à 15 sont réservés à zéro. Une option hors de sa plage fait refuser le lot ; une option absente vaut 0. La version 1 reste lisible (options nulles). Formats : `png` (0), `jpeg` (1).

Le site construit le lot dans le navigateur (`apps/web/src/workloads/batch.ts`, même format) à partir
de plusieurs fichiers ou d'un seul `.zip` (`zip.ts` : contenus stockés ou deflate, sans ZIP64 ni
chiffrement). Les fichiers sont triés par nom ; un fichier d'un mauvais type est ignoré.

Non prévu pour l'instant : rognage, normalisation du volume, WebP.

## Textes : `embedding/e5-small`

Entrée : `passages.txt`, un passage par ligne, UTF-8. Les lignes vides sont ignorées ; 8 Mo au plus
(`max_input_bytes`). Le fragment est une plage de passages (paramètres `first` et `count`, u32
little-endian en en-tête). Le résultat est, pour chaque passage dans l'ordre, un vecteur de 384 `f32`
little-endian (1 536 octets). Un passage de plus de 512 tokens est tronqué.

- Vérification : `sampling_tolerance`. Un vérificateur recalcule un échantillon de passages (tiré
  avec `samples_needed`) et compare chaque vecteur par cosinus à `cos_min` = 0,999. Un cosinus absent
  ou non fini est un rejet.
- Le site construit le fichier (`apps/web/src/workloads/embedding.ts`, `passagesFile`) et refuse un
  texte vide ou trop gros avant envoi.
- Le nœud doit avoir ONNX Runtime 1.31.1 à côté du binaire (`runtime_library` dans
  `crates/sil-node/src/cli.rs`). Sans lui, le nœud n'offre pas le workload et journalise
  `embedding non offert par ce nœud`.

## Limite connue

La spec signée du job garde les champs `width` et `height` (protocole). Un workload qui aurait
besoin d'autres paramètres demande une évolution du format de la spec et des vecteurs de test.
