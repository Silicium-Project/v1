# ADR 0001 — Comité de coordination par job

**Statut :** proposé.

## Contexte
V0 : un publisher unique par tâche (point de défaillance, finalisation sans preuve de quorum).

## Décision
Le requester signe un `JobSpec` listant n coordinateurs et un seuil m **strictement supérieur
à n/2**. Une décision (claim retenu, vérificateur retenu, finalisation) existe quand m membres
distincts ont émis le même vote : chaque vote est un événement ordinaire de la chaîne du
membre (pas de signature agrégée). La majorité stricte rend deux décisions contradictoires
impossibles. Pas de BFT global.

**Mise à jour (phase 2)** : la rotation d'époque prévue initialement n'est pas implémentée.
La liveness exige m membres joignables ; en partition l'attente est sûre. La rotation reste
à étudier avec la phase 4 (coordinateur).

## Alternatives
- Publisher unique + reprise manuelle : simple, SPOF.
- Consensus global (BFT) : lourd, inutile pour des jobs indépendants.

## Conséquences
Plus de messages par job ; en partition, l'attente est sûre (jamais de finalisation forcée).
Valeurs de (n, m) à fixer par mesure en phase 4.
