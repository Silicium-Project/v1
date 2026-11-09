# Architecture V1

## Principe

Un requester signe un `JobSpec` (wallet). Des workers calculent, des vérificateurs
attestent, les décisions sont des règles sur le journal (sans comité, ADR 0007). La finalisation
porte un **certificat** d'attestations que tout pair revalide. Le règlement on-chain est
lié au même `job_id = H(JobSpec)`.

## Dossiers

| Dossier | Rôle |
|---|---|
| `contracts/` | Schémas versionnés et vecteurs de test partagés Rust/Go/TS |
| `crates/` | Workspace Rust, un crate par responsabilité (`sil-*`) |
| `chain/` | Programme Anchor et tests localnet |
| `services/api/` | API Go : comptes, jobs, registre machines |
| `apps/web/`, `apps/desktop/` | Interface web, application Tauri |
| `infra/` | Déploiement par artefacts signés |
| `tools/` | Vérifications de dépôt (taille, dépendances), e2e |

## Crates (ordre de dépendance)

`sil-types` → `sil-crypto`, `sil-codec`, `sil-config`, `sil-telemetry` →
`sil-ledger` (+ `-store`, `sil-quota`, `sil-attest`) → `sil-content`, `sil-models`, `sil-net` → `sil-transfer` (colle réseau + stockage) →
`sil-sampling`, `sil-workload-api` (+ `sil-wl-sha256`, `sil-wl-render`), `sil-sandbox`,
`sil-reputation`, `sil-chain-core`, `sil-settlement` → `sil-control-api` → `sil-node` (voir [node.md](node.md)).

Dépendances en DAG : un crate n'importe jamais un crate situé après lui.

## Règles

- Fichier ≤ 300 lignes ; aucun état global ; config typée unique ; erreurs typées.
- Tout ce qui est signé utilise l'encodage canonique avec domaine de signature.
- Décisions structurantes : [adr/](adr/).
