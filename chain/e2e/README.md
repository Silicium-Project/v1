# chain/e2e — flux complet sur validateur local

Un requérant, trois membres du comité, un vérificateur : de la publication d'un job au règlement
on-chain. Les nœuds sont de vrais `sil_node::Node` (libp2p sur localhost, API de contrôle, moteur
de rendu de test pour `rendu/blocs`), branchés sur le programme d'escrow Solana.

Scénario (`tests/full_flow.rs`) :

1. le requérant ouvre l'escrow, dépôt = budget total de la spec ;
2. il publie la spec signée sur l'API d'un nœud ;
3. le comité calcule, le vérificateur contrôle, le comité signe le règlement ;
4. chaque fragment est réglé on-chain.

Contrôles après règlement : `paid` = dépôt ; le gain de chaque nœud = ses unités × tarif ; l'escrow
ne perd que ce qu'il paie.

Jobs : `calcul/sha256` (20 000 octets, 2 fragments) puis `rendu/blocs` (64 × 40 pixels, 3 fragments).

Le rendu passe par `sil-wl-render::ExternalRenderer` : le moteur de référence
(`sil-render-reference`) est lancé dans `sil-sandbox`. `chain/scripts/localnet.sh` le construit et
règle `SIL_RENDER_ENGINE` sur son chemin.

## Lancer

Depuis WSL, avec les outils Solana : `bash chain/scripts/localnet.sh` (voir `chain/README.md`).
Le script démarre le validateur, puis lance les tests de `chain/adapter`, puis ce flux.

Lancement seul, validateur déjà en route :

```bash
cd chain/e2e
SIL_RPC_URL=http://127.0.0.1:8899 SIL_ESCROW_PROGRAM=<id> \
  cargo test --tests -- --include-ignored --nocapture --test-threads=1
```

Ce crate est hors workspace (dépendances Solana lourdes), comme `chain/adapter` et `chain/node`.
