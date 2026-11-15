# ADR 0010 — Retenue sur la récompense des vérificateurs

**Statut** : décidée, non implémentée. Change le programme Solana : mise à jour de tous les nœuds.

## Contexte

Un même propriétaire peut contrôler plusieurs clés. Il peut les faire voter ensemble comme
vérificateurs et valider ses propres résultats. Aujourd'hui rien ne coûte à une clé qui vote mal.

## Décision

Une caution par vérificateur, prélevée sur ses gains et non déposée à l'avance.

1. **Part retenue** : 50 % de la récompense de vérificateur de chaque vote est mise en réserve
   on-chain, au nom de la clé.
2. **Libération** : la réserve d'un vote est libérée à la première échéance atteinte parmi :
   - 20 règlements après le dernier vote de la clé ;
   - 24 h après le dernier vote de la clé.
3. **Pénalité** : si un vote de la clé est jugé faux (désaccord avec le résultat retenu, ADR 0008),
   la réserve de la clé est confisquée, en tout ou en partie. Le barème exact reste à fixer.
4. **Sélection** : les vérificateurs d'un fragment sont tirés de façon imprévisible pour l'auteur du
   résultat. Le tirage utilise une valeur que personne ne choisit seul (hash de slot Solana).
5. **Nombre de vérificateurs** : à fixer. Aucune valeur n'est retenue.

## Conséquences

- Un propriétaire qui multiplie les clés doit immobiliser 50 % de leurs gains de vérification pendant
  la période de libération, et risque de les perdre.
- Une clé neuve n'a pas de caution à déposer : elle l'accumule. Le coût d'entrée est faible, mais la
  première période de gains est bloquée.
- **Format** : nouveau compte de réserve par clé dans le programme Solana, nouvelles instructions de
  libération et de confiscation. Déploiement incompatible : tous les nœuds changent en même temps.
- Une clé inactive récupère sa réserve par l'échéance de 24 h.

## Points ouverts

- Définition de « règlement » comptée pour les 20 (tous les règlements du programme ou ceux du
  vérificateur seul).
- Barème de confiscation (totale ou proportionnelle).
- Nombre de vérificateurs par fragment.

## Correctif de revue : libération et confiscation par job

- **Libération** : le compteur ne compte que les votes **propres** du vérificateur, depuis sa plus
  ancienne réserve en attente (`pending_since`). N'importe qui ne peut plus le faire avancer en
  réglant d'autres jobs. Le délai de 24 h après le dernier vote reste en place.
- **Par job** : chaque `SettledFragment` porte `held` (la réserve de chaque approbateur sur ce
  fragment). Une pénalité ne consomme que ce que cet escrow a retenu pour le vérificateur ; une
  libération (`ReleaseReserve`, avec les escrows en comptes) ne sort que ce que ces escrows
  détiennent. Plus de confiscation croisée entre jobs.
- Le compte de stats global est supprimé.
