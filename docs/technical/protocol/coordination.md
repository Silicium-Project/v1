# Coordination (`sil-coordinator`, `sil-reputation`)

## L'automate

`Coordinator::decide(ledger, réputation, job, maintenant)` retourne les votes que **ce
membre** doit émettre. Il ne signe ni n'envoie rien. Pour chaque fragment non terminal :

1. **Claim** : s'il y a une tentative ouverte et que ce membre n'a pas encore voté pour elle,
   il vote pour le meilleur claim.
2. **Vérificateurs** : si des places sont ouvertes, il vote pour les meilleures offres de
   vérification, moins ses votes déjà en attente.
3. **Finalisation** : si le fragment est `Verified` et que le journal contient assez
   d'approbations de vérificateurs retenus, il vote `Finalize` avec le certificat.

Classement des offres : meilleure réputation, puis la plus ancienne, puis le plus petit
identifiant. C'est un ordre total : des membres qui voient les mêmes offres choisissent
pareil, donc leurs votes convergent et atteignent le seuil.

**Idempotence** : un vote déjà émis n'est jamais réémis. C'est indispensable : seul le
premier vote d'un membre sur un emplacement compte dans le journal (`ledger.md`).

**Pas de finalisation sans preuve** : `Finalize` n'est proposé qu'avec les attestations réelles
(`Ledger::approvals`) ; sans approbation d'un vérificateur retenu, l'automate n'émet rien
(testé, y compris avec un vérificateur retenu mais muet).

Le test `the_committee_alone_drives_a_job_from_claim_to_finalization` déroule un job complet
avec trois coordinateurs, un worker et un vérificateur, sans aucun vote manuel.

## Vivacité des votes : fenêtre et alignement

Les votes sont **irrévocables** (seul le premier vote d'un membre compte, `ledger.md`). Si deux
membres votent chacun pour « leur » claim avant d'avoir vu celui de l'autre, aucun claim n'atteint
plus jamais le seuil quand seuls `threshold` membres sont vivants. Un test de bout en bout a
reproduit ce blocage (environ une exécution sur trois, avec 2 membres vivants sur 3). Deux règles le réduisent :

1. **Fenêtre d'observation** (`Coordinator::with_window`, 1 s dans le nœud) : un membre ne vote que
   lorsque le plus ancien claim ouvert a cet âge, pour que tous aient vu les mêmes offres.
2. **Alignement** : s'il existe déjà des votes d'autres membres pour un claim qu'il connaît, il vote
   pour le plus voté (puis le mieux classé) au lieu de son propre choix. Les votes pour un claim
   inconnu sont ignorés (un membre malveillant ne peut pas bloquer les autres).

3. **Chef de tour** : `leader_index(job, fragment, tentative, n)` désigne un membre (hash de
   l'emplacement mod n, identique chez tous). Les autres attendent son vote, et que son claim leur
   soit parvenu, jusqu'à `leader_wait` (3 s) après la fenêtre d'observation, puis votent d'eux-mêmes
   (chef muet ou en panne : la vivacité ne dépend pas de lui). Avec l'alignement, tous votent
   alors pour le claim du chef.

Résultat : 7 exécutions consécutives de la suite de bout en bout sans échec. Ce n'est **pas** une
garantie : un chef muet qui revient après l'échéance, ou des vues différentes, peuvent encore
diviser le vote. Une vraie solution (tour avec expiration, rotation du comité) reste à concevoir.

## Réputation (`sil-reputation`, ADR 0003)

| Règle | Effet |
|---|---|
| Scopes de confiance | seules les observations de requesters que ce nœud a choisis comptent |
| Pas d'auto-évaluation | une observation où le worker est le requester ou membre du comité est ignorée |
| Poids borné | un succès pèse `1 + log10(1 + valeur)`, au plus 2 ; valeur = unités du fragment × tarif par unité |
| Décroissance | demi-vie de 7 jours : un score se maintient, ne se capitalise pas |
| Probation | un inconnu a un score de 0,5 et une tâche à la fois ; +1 par succès, au plus 8 |
| Suspension | échecs dominants (> succès + 3) : zéro tâche |
| Mémoire bornée | 100 000 workers au plus, les entrées périmées sont purgées |

V0 comptait les tâches de **n'importe quel** publisher : s'auto-publier, s'attribuer et
s'auto-vérifier gonflait le score. Ici ces observations sont ignorées (testé).

## Limites connues

- **Pas de rotation du comité** ni de coordinateur de secours : la vivacité exige `threshold`
  membres joignables. En partition, l'attente est sûre (jamais de finalisation forcée).
- **Sybil non éliminé** : sans coût d'entrée, une clé jetée puis recréée retrouve la probation.
  Le dégât est borné (une tâche à la fois), pas supprimé. Un stake léger reste à trancher.
- **Auto-vérification supprimée** : le journal interdit au claimant de vérifier son résultat.
  Un réseau d'**un seul worker** ne peut donc pas vérifier ses jobs (V0 l'autorisait avec la
  « règle du worker seul », qui reposait sur des rôles auto-déclarés). Il faut au moins deux
  nœuds de calcul/vérification.
- L'automate n'implémente pas encore la prise en compte des vérificateurs « chauds » (modèle déjà
  chargé) de V0.
- Les observations de réputation ne sont pas encore alimentées automatiquement depuis le
  journal : c'est le rôle de `sil-node` (phase 6).
