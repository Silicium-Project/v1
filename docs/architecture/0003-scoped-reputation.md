# ADR 0003 — Réputation scopée et anti-Sybil

**Statut :** proposé, question ouverte (stake léger ou période probatoire).

## Contexte
V0 : la réputation compte les tâches de n'importe quel publisher ; un attaquant s'auto-publie
et se vérifie pour gonfler son score, et une nouvelle clé remet le score à zéro sans coût.

## Décision
Réputation calculée par comité/requester, avec décroissance, pondérée par la valeur vérifiée
indépendamment. Auto-publication et auto-vérification exclues du calcul. Les nouvelles clés
passent une période probatoire à charge limitée.

## À trancher
Stake léger Solana vs période probatoire seule.

**Mise à jour (phase 4)** : implémenté dans `sil-reputation` (scopes de confiance, auto-évaluation
ignorée, poids borné, demi-vie 7 jours, probation d'une tâche, mémoire bornée). Le coût d'entrée
(stake) reste ouvert : une clé jetée retrouve la probation. Voir protocol/coordination.md.
