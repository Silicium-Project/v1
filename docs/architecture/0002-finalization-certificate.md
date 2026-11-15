# ADR 0002 — Certificat de finalisation

**Statut :** proposé.

## Contexte
V0 : `TaskFinalized` est accepté si le résultat existe, sans vérifier le seuil d'approbations.

## Décision
`TaskFinalized` embarque ≥ k attestations signées de vérificateurs acceptés. Tout réplica
revalide le certificat ; sans certificat valide l'événement est rejeté. Un rejet porte une
preuve (blocs divergents signés), pas un booléen.

## Conséquences
Un coordinateur compromis ne peut pas fabriquer une finalisation. Le certificat est aussi
la racine de preuve présentée au programme Solana.

**Mise à jour (phase 2)** : le rejet porte une référence de preuve (`evidence`) mais le
journal ne la vérifie pas (propre au workload). Pour qu'un vérificateur isolé ne puisse pas
bloquer un résultat, V rejets (V = `verifications_required`) sont nécessaires pour rejeter,
et V approbations plus V rejets donnent l'état `Disputed`. Voir protocol/ledger.md.
