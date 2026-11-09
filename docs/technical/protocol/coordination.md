# Coordination (sans comité, ADR 0007)

Il n'y a plus de comité de coordinateurs. Aucun nœud ne vote pour un autre. Chaque décision est une
règle déterministe sur le journal du job (ADR 0005), et le règlement on-chain est signé par les
vérificateurs eux-mêmes (ADR 0007).

## Qui fait quoi

| Étape | Qui | Règle |
|---|---|---|
| Publication | le requester | spec signée ; pas de comité ni de seuil dans la spec |
| Claim | tout nœud `worker` | le premier claim valide l'emporte (ordre total `(horodatage, identifiant)`) |
| Bail | le claimant | renouvelé tant qu'il calcule (`Renew`), expire sinon |
| Offre de vérification | tout nœud `verifier` | le nœud connaît le workload ; le claimant ne s'offre jamais |
| Vérificateurs retenus | le journal | les `k` premières offres valides, d'auteurs distincts, hors claimant |
| Verdict | chaque vérificateur retenu | `Attest` signé ; une approbation porte sa signature de règlement |
| Finalisation | tout nœud | `Finalize` avec `k` approbations distinctes (certificat) |
| Règlement on-chain | tout nœud (outbox) | les `k` signatures des approbateurs, chacun éligible on-chain |

## Réputation

- Un compte de réputation par clé, créé par `RegisterVerifier` à 5 000. Un règlement exige un score
  ≥ 2 500 pour chaque approbateur (ADR 0006).
- Un règlement approuvé ajoute 100 au score de chaque approbateur, plafonné à 10 000.
- **Pénalité du requester (ADR 0008).** Le requester peut pénaliser un approbateur d'un fragment réglé : -1 000, une fois par fragment. Trois fautes sortent un vérificateur de l'éligibilité.

## Mode solo

Supprimé. Un claimant ne vérifie jamais son propre résultat. Un job à `k` vérifications demande donc
au moins `k + 1` nœuds (ou `k` vérificateurs et un worker distinct).

## Vivacité

Un job avance tant qu'un worker et `k` vérificateurs éligibles sont en ligne. Aucun membre n'est
indispensable : un nœud arrêté ne bloque plus les autres.

## Limites connues

- Pas de contestation par preuve : la pénalité repose sur le requester (ADR 0008), qui peut se tromper ou être complice.
- Pas de vote on-chain individuel comme en V0 : le règlement agrège les signatures des approbateurs.
- L'enregistrement on-chain (`register_verifier_ix`) est fait par le payeur du règlement, dans la même transaction.
