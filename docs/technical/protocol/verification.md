# Vérification des résultats

Un résultat n'est jamais cru sur parole : V vérificateurs indépendants (`verifications_required`)
doivent l'approuver (`ledger.md`). Ce document décrit **comment un vérificateur juge**.

## Contrat d'un workload (`sil-workload-api`)

`split` (découper), `execute` (calculer), `verify` (juger le résultat d'un autre), `block_count`,
`combine`, `limits`, `policy`. Un workload est enregistré dans un `Registry` explicite (aucun
état global). Un rejet est toujours accompagné d'une `Evidence` (bloc, engagé, recalculé,
raison) dont le hash va dans l'attestation de rejet.

| Politique | Le vérificateur… | Quand |
|---|---|---|
| `Recompute` | refait tout le calcul | résultat court ou calcul peu coûteux (`calcul/sha256`) |
| `Sampling { min_cheat_fraction, target_detection }` | refait un échantillon de blocs | calcul long découpé en blocs (`rendu/blocs`) |

Avec V = 2 et `Recompute`, deux machines refont le calcul : c'est la **redondance**, sans
mécanisme séparé (le journal retient un seul claimant par tentative).

## Graine d'échantillonnage (`sil-sampling`)

Le vérificateur signe `(tâche, tentative, hash du résultat)` dans le domaine `Sampling` ; le
hash de la signature est la graine. Ed25519 est déterministe :
- le vérificateur **ne peut pas retirer** jusqu'à obtenir un tirage favorable ;
- n'importe qui **revérifie** la graine avec sa clé publique ;
- le worker ne peut pas l'**anticiper** (il faudrait la clé du vérificateur) ;
- le comité choisit *qui* vérifie mais pas la graine de chacun.

Chaque vérificateur tire son propre échantillon : les échantillons s'ajoutent. Pour passer
inaperçu il faut contrôler **tous** les vérificateurs retenus. (V0 : graine = hash de
l'événement d'acceptation, que le publisher pouvait ajuster.)

## Taille de l'échantillon

`samples_needed(blocs, min_cheat_fraction, cible)` : plus petit échantillon qui détecte, avec
probabilité ≥ cible, un tricheur falsifiant au moins cette fraction des blocs (loi
hypergéométrique, vérifiée par test). Pour une cible de 99 % :

| Fraction falsifiée | 100 blocs | 1000 blocs |
|---|---|---|
| 20 % | 19 | 21 |
| 10 % | 36 | 43 |
| 5 % | 59 | 86 |
| 1 % | 99 | 368 |

La taille dépend de la **fraction** visée, presque pas de la taille du travail. Le workload de
rendu vise 20 % à 99 % (`min_cheat_fraction = 0.2`).

## Ce que ça ne protège PAS

C'est une dissuasion statistique, **pas une preuve de calcul** :
- un seul bloc falsifié sur 100 est détecté avec une probabilité de **19 %** par vérificateur
  (34 % avec deux vérificateurs indépendants) ; testé dans `sil-wl-render` ;
- V0 : un bloc sur 100 avec 8 blocs contrôlés = 8 % ; avec 20 blocs falsifiés = 84 % (ici ≥ 99 %).

Si l'enjeu d'un bloc isolé est élevé, utiliser `Recompute` ou baisser `min_cheat_fraction` (coût
proportionnel). Un résultat approuvé n'est pas « exact », il est « non contredit ».

## Rendu par blocs (`sil-wl-render`)

Trois contrôles, dans l'ordre :
1. taille des pixels et nombre de milestones cohérents ;
2. **chaque bloc de pixels livré a bien le hash engagé** (V0 ne le vérifiait pas : on pouvait
   engager des hash honnêtes et livrer d'autres pixels) ;
3. les blocs de l'échantillon sont recalculés et comparés aux milestones.

Le moteur est injecté (`BlockRenderer`) et doit être déterministe à l'octet près.

## Limites connues

- Les workloads **embeddings** et **LLM** de V0 ne sont pas portés. L'inférence flottante n'est
  pas bit-à-bit reproductible entre machines : leur vérification demande une politique de
  tolérance (comparaison d'embeddings, canaris) qui reste à concevoir.
- Le moteur de rendu de V0 n'est pas porté : seul le cadre de découpe/vérification l'est.
- La preuve (`Evidence`) n'est pas elle-même revérifiée par les pairs : un vérificateur
  malveillant peut rejeter avec une fausse preuve. Il faut V rejets pour rejeter une tentative
  (`ledger.md`), et le contenu de la preuve peut être rejoué par un tiers.
