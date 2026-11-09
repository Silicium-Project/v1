# chain — programme Solana de l'escrow (V1)

**État : écrit et testé sur un validateur local.** Règles dans `crates/sil-chain-core` ; programme
dans `chain/program` ; adaptateur dans `chain/adapter`. Spécification : `docs/technical/protocol/settlement.md`.

## Ce qui existe

| Dossier | Contenu |
|---|---|
| `program/` | programme natif (`solana-program` 3), fine enveloppe autour de `sil-chain-core` : `open`, `top_up`, `settle` (précompilés Ed25519 des approbateurs), `refund` |
| `adapter/` | `SolanaChain` : implémente `sil_settlement::Chain`, constructeurs de transactions, traduction des erreurs |
| `scripts/localnet.sh` | compile le programme SBF, démarre `solana-test-validator`, lance les tests de l'adaptateur |
| `node/` | `silicium-node` avec la chaîne Solana : même démarrage que `sil-node`, plus `[chain]` dans la configuration |
| `e2e/` | flux complet sur validateur local : requérant, trois workers, un vérificateur, règlement on-chain (voir `e2e/README.md`) |

## Lancer les tests sur un validateur local

Les outils Solana ne tournent pas sous Windows natif : on utilise WSL (Ubuntu 22.04).

Installation (une fois, dans WSL) :

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"
```

Lancement (depuis le dépôt, dans WSL) :

```bash
bash chain/scripts/localnet.sh
```

Résultat attendu : `5 passed` pour `chain/adapter` (voir `chain/adapter/README.md`), puis `1 passed` pour
`chain/e2e` (flux complet : requérant, workers, vérificateur, règlement on-chain, voir `chain/e2e/README.md`).

## Choix faits (à relire)

- **Programme natif, pas Anchor** : moins de dépendances, pas de macro cachée, et l'interface est
  un enum partagé avec l'adaptateur. Le plan prévoyait Anchor ; changement assumé.
- **Format binaire bincode** pour l'état et les instructions. Le décodage CBOR générique dépassait
  la pile autorisée en SBF (4 Ko par fonction) : le build échouait.
- **Compte d'escrow gardé après remboursement** (état `closed`), pour empêcher de rouvrir le même job.
  Le loyer reste bloqué.
- **Paiement en SOL** (lamports), pas en jeton SPL.
- **256 fragments au plus** par escrow on-chain (le loyer du compte croît avec le nombre de fragments).
- **Frais de règlement payés par le job** : `SETTLEMENT_FEE` (10 000 lamports) par règlement, versé au nœud qui soumet, pris sur le dépôt du requester.
- **Tarifs par unité** : le dépôt se calcule à l'ouverture (unités × tarifs + frais par fragment). Les montants payés suivent les unités réelles de chaque fragment (`crates/sil-chain-core/src/units.rs`).

## Limites

- **Pas audité.** Audit externe obligatoire avant tout fonds réels.
- **Clé d'autorité de mise à jour : celle du validateur local.** Le multisig prévu n'est pas en place.
- **Pas de test sur un réseau public** (devnet/testnet).
- Le binaire `sil-node` (sans chaîne) refuse une configuration qui demande Solana : utiliser `chain/node`.
- Tarifs par unité : une unité = un octet pour `calcul/sha256`, un pixel pour `rendu/raytracer`. Pas de bonus de qualité ni de malus de retard. Un autre workload est refusé à l'ouverture.
- Non testé : 429 et délais réels, expiration de blockhash réelle, worker sans SOL (la logique est
  couverte par `FakeChain` dans `sil-settlement`).

## Faire tourner un nœud qui règle sur Solana

```bash
cd chain/node && cargo build
./target/debug/silicium-node --config node.toml --roles worker,verifier
```

Section à ajouter à `node.toml` (voir `docs/user/configuration.md`) :

```toml
[chain]
rpc_url = "http://127.0.0.1:8899"
program_id = "<identifiant du programme, base58>"
payer_keypair = "payer.json"   # fichier JSON de clé Solana ; ce compte paie les frais
```

Le compte payer doit avoir un peu de SOL : sans cela, il ne soumet rien (`NoFunds`).
