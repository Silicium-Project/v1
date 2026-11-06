# sil-settlement-solana

Adaptateur Solana de `sil-settlement` (`impl Chain`) et constructeurs de transactions de l'escrow.

Dépend de : `sil-chain-core` (règles et format), `sil-settlement` (trait `Chain`), Solana 3.x.
Hors du workspace du nœud : les dépendances Solana sont lourdes ; le nœud l'utilisera plus tard.

## Tests contre un vrai validateur

Les scénarios sont dans `tests/localnet.rs`, marqués `#[ignore]`. Ils se lancent avec
`chain/scripts/localnet.sh` (dans WSL, voir `chain/README.md`) : le script compile le programme,
démarre `solana-test-validator` avec lui, puis lance les tests.

| Test | Vérifie |
|---|---|
| `escrow_lifecycle_on_a_local_validator` | ouverture, règlement avec 2 signatures du comité (précompilés Ed25519), paiement du claimant et des vérificateurs, double règlement refusé, signature insuffisante refusée, signature d'un non-membre refusée, dépôt complémentaire, remboursement avant échéance refusé |
| `an_expired_escrow_refunds_its_balance_and_cannot_be_reopened` | remboursement après échéance et délai de grâce, compte gardé fermé, job non rouvrable |
