# ADR 0013 — Réputation multi-dimensionnelle bayésienne et attribution probabiliste des claims

**Statut** : partiellement implémentée. Remplace et étend `sil-reputation` (score scalaire local,
ADR 0003) et le score de vérificateur on-chain (ADR 0006) pour tout ce qui ne touche pas directement
l'argent. Les pénalités et pondérations qui touchent l'argent restent gouvernées par les ADR 0006,
0008, 0010, 0012 : cet ADR ne les modifie pas.

Fait : phase 1 (primitives bayésiennes déterministes, crate `sil-bayes` — `BetaPosterior`,
`NormalGammaPosterior`, `exp`/`ln`/`√`/`Φ` en entiers fixes) et phase 2 (vecteur de ressources par
workload, `sil-workload-spec::Resources`, dérivé du descripteur statique du workload plutôt qu'un
champ signé par job — voir § 3). Reste à faire : phases 3 à 10, bloquées pour les phases 3 à 7 par
l'ADR 0009 (non implémenté) et par les valeurs numériques listées en Points ouverts.

## Contexte

Le réseau n'a aujourd'hui qu'un score scalaire par worker, local à chaque nœud (`sil-reputation`),
et un score de vérificateur on-chain à une seule dimension (ADR 0006). Aucun des deux ne capture la
diversité réelle des machines (vitesse réseau, calcul, stockage, inférence de modèle) ni les
compétences distinctes (délégation de tâche, détection de fausse vérification, rétention de
données). Le besoin, exprimé par le requester :

> Les scores de tout agent du réseau doivent être subdivisés en spécialisation, chaque worker
> possède l'intégralité des scores, et apprend avec le temps à se spécialiser par ajustement
> bayésien des valeurs. Les scores intègrent entre autres : vitesse réseau (ping, temps
> d'acceptation, débit montant/descendant...), taux de succès, vitesse de calcul, qualité de
> délégation des tâches, qualité de vérification (détecter une fausse vérification), capacité à
> stocker correctement sur la durée (vitesse lecture/écriture, type de disque, taux de rétention
> après audit). Le réseau doit aussi évaluer des capacités spécifiques (inférence de modèle,
> vitesse GPU, RAM/VRAM...), idéalement sans nommer explicitement les types de tâche.

Cette discussion a tranché, point par point, la forme que ce système doit prendre. Cet ADR les
rassemble.

## Décision

### 1. Les scores sont partagés, dérivés du journal — jamais une opinion locale

Un score qui ne donne pas le même résultat sur tous les nœuds ne peut gouverner aucune décision qui
touche l'attribution ou l'argent (ADR 0005 pose déjà ce principe pour la réputation de vérificateur).
Chaque dimension de score est donc une fonction **déterministe** des événements signés du journal :
mêmes événements, même ordre total, même résultat partout. Un nœud peut garder une opinion locale
pour des choix sans enjeu (ordre d'interrogation des pairs), mais elle n'entre jamais dans une
attribution, une pénalité ou un paiement.

« Chaque worker possède l'intégralité des scores » : chaque nœud reconstruit, à partir du même
journal, le score de tout agent sur toutes les dimensions. Il n'y a pas de table centrale.

### 2. Les mesures sont vérifiées, jamais déclarées

Un worker ne doit jamais pouvoir faire monter son propre score en déclarant une valeur. Deux sources
valides :

- **Mesure dérivée du résultat d'un vrai job** : débit observé = unités traitées (barème
  `sil-chain-core::units`) divisé par une durée **ancrée on-chain** (slot Solana, ADR 0009), jamais
  l'horloge locale du worker ni celle du pair qui l'observe seul.
- **Sonde ou défi à réponse connue, lancé par un pair** : pour les dimensions qu'un vrai job ne
  couvre pas directement (latence réseau pure, débit brut, lecture/écriture disque). Le pair qui
  sonde signe l'observation comme un événement du journal.

Aucune valeur auto-déclarée (VRAM annoncée, type de disque annoncé) n'entre dans un score tant
qu'elle n'est pas corroborée par une mesure de ce type.

### 3. Spécialisation par vecteur de ressources, pas par type de tâche nommé

Pour rester généraliste (« sans mentionner explicitement les types de tâche »), chaque job porte un
vecteur de ressources qu'il sollicite (profil GPU/CPU, RAM/VRAM requise, intensité réseau, intensité
stockage — des grandeurs déjà présentes ou dérivables de `EscrowTerms`/`JobSpec`, pas une étiquette de
workload). Le réseau apprend la relation **vecteur de ressources → résultat observé** par agent, via
un modèle hiérarchique : un a priori global par dimension, affiné par agent à mesure que ses
observations s'accumulent. Un worload nouveau n'a besoin d'aucune nouvelle catégorie : il n'est qu'un
nouveau point dans l'espace des vecteurs déjà modélisé.

**Implémenté (phase 2), avec une simplification délibérée** : plutôt qu'un nouveau champ signé par
job (qui aurait changé le format de l'escrow on-chain, ADR 0009 n'étant pas encore en place pour
amortir une rupture de plus), le vecteur de ressources (`compute`, `memory`, `network`, `storage`,
0 à 100) est porté par le **descripteur statique du workload** (`contracts/workloads/*.json` →
`sil-workload-spec::Resources`, un par workload, jamais un agrégat). La couche de score ne lit jamais
l'identifiant du workload, seulement ce vecteur : la contrainte « sans nommer le type de tâche » est
respectée au niveau où elle compte (le scoring), au prix d'une granularité par workload plutôt que
par instance de job. Revoir ce choix si un jour un même workload doit pouvoir solliciter des profils
très différents selon son entrée.

### 4. Modèle bayésien : une famille conjuguée par nature de mesure

| Nature de la mesure | Modèle | Pourquoi |
|---|---|---|
| Taux (succès, fausse vérification, rétention après audit) | **Beta-Bernoulli** | Borné [0,1] par construction ; exact même à faible échantillon (dizaines d'observations) ou taux proche de 0/1. La gaussienne a été écartée pour ce cas : elle déborde de l'intervalle et sous-estime la queue (comparaison tracée en séance, Beta(20,2) vs gaussienne de même moyenne/variance). |
| Mesure continue (débit, latence, vitesse de calcul) | **Normal-Gamma** (gaussien à variance inconnue) | Conjugué, mise à jour en flux (`τₙ = τ₀ + n·τ`, `μₙ = (τ₀μ₀ + τ·Σxᵢ)/τₙ`), prédictive en loi de Student (robuste aux valeurs aberrantes à faible échantillon). |
| Spécialisation (lien ressources → résultat) | **A priori hiérarchique** sur les paramètres ci-dessus, par agent, informé par un a priori de population | Un agent neuf hérite de l'a priori global ; il se spécialise à mesure que ses propres observations dominent l'a priori. |

Toutes les fonctions transcendantes qu'exige ce modèle (fonction bêta incomplète régularisée pour
`P(taux > seuil)`, `Φ` pour le cas gaussien) passent par une **table figée en virgule fixe**, versée
dans le code : deux nœuds avec les mêmes entrées doivent produire le même entier, jamais un flottant
dépendant de la plateforme.

### 5. Diagrammes puissance → résultat, un par dimension

Pour les capacités matérielles (vitesse de calcul, GPU, débit, stockage), le réseau construit, par
dimension, un diagramme reliant la puissance observée au résultat observé (ADR proposé en séance,
confirmé). Ce diagramme :

- se construit sur des mesures **vérifiées** (point 2), jamais déclarées ;
- donne une **enveloppe apprise** (quantiles), pas une droite fixée a priori ;
- sert à détecter un écart entre la puissance annoncée et le résultat réel (indice de score), **pas**
  à rejeter un résultat : rejeter un calcul reste une décision de vérification (ADR 0002, 0007), pas
  une décision statistique.
- existe en plusieurs versions, une par type de réputation/dimension (calcul, stockage, réseau,
  inférence…), jamais un diagramme unique agrégeant tout.

### 6. Le score est un prior ; seules les valeurs extrêmes pèsent directement

Un score ne se calcule — et ne pèse sur une décision — qu'après un nombre minimal fixe
d'évaluations ; avant ce seuil, l'agent reste en **probation** (voir § 8) et le score ne compte pas.
Au-delà, le score sert de **prior** à la décision (attribution, quota), sauf quand la postérieure
indique une valeur extrême (fraude quasi certaine, incompétence quasi certaine) : dans ce cas précis,
le score prime la règle par défaut.

Le degré de certitude utilisé pour juger « extrême » est la probabilité postérieure elle-même
(`P(taux > seuil)` via Beta, ou le score de certitude gaussien), jamais une opinion locale.

### 7. Seuils dynamiques selon l'état du réseau — déterministes, bornés, lissés

Les seuils de déclenchement (ce qui compte comme « extrême », ce qui ouvre ou resserre un quota) se
régulent selon l'état du réseau : un réseau en défense (taux de litige élevé, chute du nombre de
vérificateurs actifs) resserre ; un réseau appauvri (peu de workers, peu de vérificateurs) relâche.

Conditions retenues :

- **L'état du réseau est lui-même une fonction déterministe du journal** (jamais une mesure locale),
  construite sur des signaux difficiles à manipuler artificiellement (taux de litige **rapporté au
  nombre de vérificateurs actifs**, pas un simple compte d'événements).
- **Régulation de base linéaire bornée (clamp)**, avec plancher et plafond fixes codés en dur par
  catégorie, complétée si besoin par une **sigmoïde en table figée** (virgule fixe) quand une
  transition plus lisse est utile. Toute forme à base d'exponentielle passe par une table figée,
  jamais `f64::exp` en direct.
- Un **lissage temporel** (pas de saut brutal d'un tour à l'autre) est obligatoire.

### 8. Démarrage d'un agent neuf : probation puis exploration contrôlée

Un agent sans historique reste en probation (un seul claim actif, comme l'actuel `max_parallel` de
`sil-reputation`). Passé le nombre minimal d'évaluations (§ 6), son quota et son poids dans les tirages
s'ouvrent progressivement. L'exploration de sa spécialisation (quelles dimensions il sert bien) suit
un bandit de type Thompson : il reçoit, de temps en temps, des claims hors de son profil appris pour
mettre à jour sa postérieure sans l'affamer ni le favoriser indéfiniment sur un seul profil.

### 9. Attribution des claims : n loteries indépendantes, pas une course

Chaque claim subit son propre tirage indépendant — il n'y a **pas** de course entre claims à
départager par un tiers ni par une loterie adversariale unique. Pour un claim donné :

```
accepté si hash(graine_slot ‖ claim_id) < p(score)
```

- **Graine** : un slot Solana postérieur à l'émission du claim (ADR 0009). Personne ne la choisit ;
  un tiers tireur a été écarté (il ajoute un point de confiance — censure possible par omission —
  sans rien apporter que la graine de slot ne donne déjà).
- **`p(score)`** : construit à partir de la postérieure (§ 6), avec un **plancher fixe** laissant une
  chance non nulle au plus petit agent (évite qu'un score bas n'exclue totalement, ce qui
  contredirait le principe « prior sauf extrême »).
- **Plusieurs claims acceptés sur le même fragment** : le premier accepté dans l'ordre total
  (`issued_at_ms, id`, ADR 0005) prend le fragment ; les autres restent éligibles au tirage suivant
  pour une autre tentative.
- **Liveness** : `p` croît avec le temps d'attente du fragment, jusqu'à 1 après une durée fixe, pour
  qu'un fragment ne reste jamais bloqué faute de tirage favorable.

### 10. Identification et ciblage des fragments (parallélisme sans collision)

Pour exploiter le parallélisme sans coordinateur ni collisions coûteuses :

- **Ordre de préférence déterministe par worker** : `hash(clé_worker ‖ job ‖ fragment)` trie les
  fragments ouverts pour ce worker. Des workers distincts visent ainsi, sans se coordonner, des
  fragments différents en priorité.
- **Balayage par rang** : un worker inactif sur son fragment préféré (déjà pris, ou refusé par le
  tirage § 9) passe au suivant de son ordre.
- Une collision reste possible mais rare, et sans coût : un seul claim gagne par (fragment,
  tentative), les autres ne payent rien (ADR 0005, déjà en place).

### 11. Limiter les claims multiples sans coût d'entrée

Un coût fixe par claim est rédhibitoire pour le tout premier claim d'un agent sans historique
(confirmé en séance). La limite retenue combine, sans jamais exiger de paiement initial :

- **Quota de claims actifs simultanés**, pas seulement par fenêtre temporelle. Un agent neuf en a un
  seul (probation, § 8). Le quota croît avec la réputation postérieure.
- **Quota dérivé du diagramme puissance → résultat (§ 5)**, une fois le seuil de confiance (§ 6)
  atteint :
  `quota = min(plafond, floor(débit_bas × fenêtre / unités_moyennes_fragment))`, où `débit_bas` est
  le **quantile bas** (pas la moyenne — un agent chanceux sur une observation ne doit pas obtenir un
  quota qu'il n'a pas prouvé) du diagramme de cet agent sur cette dimension.
- **Caution remboursable plutôt qu'un coût** pour les claims au-delà du premier : bloquée au claim,
  rendue à la vérification réussie. Nulle pour le premier claim d'un agent neuf.
- Le plafond par tâche déjà existant (`max_parallel`, `sil-reputation`) continue de s'appliquer en
  plus de ce quota.

Limite reconnue et non résolue par ce qui précède : une population de clés jetables (sybils) peut
toujours accumuler des premiers claims gratuits en parallèle sur des identités distinctes. Le quota et
la probation bornent le dégât par identité, ils ne l'empêchent pas à l'échelle d'une population de
clés. Traité comme point ouvert (§ Points ouverts).

## Conséquences

- **Format** : nouvelles dimensions d'événements signés (sondes/défis de pairs, vecteurs de
  ressources des jobs). Changement incompatible pour tout nœud qui calcule un score : déploiement
  simultané requis, comme les ADR 0009/0010/0012.
- **Dépendance renforcée à l'ADR 0009** : la graine de slot et l'ancrage des durées sont des
  prérequis directs de ce système (§ 2, § 9). Cet ADR ne peut pas être implémenté avant l'ADR 0009.
- **Coût de calcul** : les mises à jour bayésiennes et les tables figées ajoutent du calcul par nœud,
  borné et déterministe (pas de `f64` transcendant en direct).
- **`sil-reputation` est remplacé** pour tout ce qui gouverne une attribution (claims, quotas,
  spécialisation). Il peut subsister comme opinion locale pure, sans effet sur l'attribution.
- Le score de vérificateur on-chain (ADR 0006) et les pénalités (ADR 0008, 0010, 0012) ne changent
  pas : ils restent la seule voie qui touche l'argent directement.

## Points ouverts

- Nombre minimal d'évaluations avant sortie de probation (§ 6, § 11).
- Seuil de certitude codé en dur pour « valeur extrême » (§ 6) et pour l'ouverture du quota diagramme
  (§ 11), par catégorie.
- Planchers/plafonds par catégorie pour les seuils dynamiques (§ 7), et durée de lissage.
- Forme exacte de `p(score)` (§ 9) : clamp linéaire ou sigmoïde en table, et son plancher pour le plus
  petit agent.
- Durée jusqu'à `p = 1` (liveness, § 9) et durée de la fenêtre du quota diagramme (§ 11).
- Signaux exacts qui définissent un réseau « défensif » ou « appauvri », par catégorie (§ 7).
- Vecteur de ressources exact porté par un job (quelles dimensions, quelle granularité) — à définir
  avec `sil-workload-spec`.
- Protection anti-sybil à l'échelle d'une population de clés (reconnue non résolue, § 11) : coût
  d'entrée différé, preuve de travail légère, ou lien à une identité externe — à trancher séparément.
- Jeu précis de sondes/défis par dimension (réseau, disque, GPU) et leur fréquence.

## Plan d'implémentation

Ordre de dépendance : l'ADR 0009 (ancrage sur slot) est un prérequis dur des phases 3 à 7 (toute
mesure de durée et toute graine de tirage en dépend). Les phases 1 et 2 sont indépendantes et peuvent
commencer immédiatement.

1. **Fondations bayésiennes déterministes** — **fait** : crate `sil-bayes`
   (`BetaPosterior`, `NormalGammaPosterior`, module `fixed` : `exp_micro`, `ln_micro`, `sqrt_micro`,
   `phi_micro`, tous en `i128`, aucune fonction transcendante système). Tests de fermeture exacte
   (Beta(1,1) linéaire, Beta(2,1) quadratique, Beta(a,a) symétrique à 0,5) et de déterminisme.
2. **Vecteurs de ressources des workloads** — **fait**, avec la simplification du § 3 : champ
   `resources` dans `contracts/workloads/*.json`, validé et émis par `tools/workloads/generate.mjs`
   vers `sil-workload-spec::Resources` (Rust), `workloadspec.Resources` (Go) et `WorkloadSpec`
   (TypeScript, champ `resources`).
3. **Sondes et défis signés** — nouveau type d'événement de journal (observation d'un pair sur un
   autre : latence, débit, lecture/écriture disque), signature, règles d'acceptation
   (indiscernabilité d'un vrai job, fréquence).
4. **Diagrammes puissance → résultat** — construction par dimension à partir des mesures vérifiées
   (phases 1 et 3), calcul des quantiles bas, un diagramme par dimension (pas un agrégat unique).
5. **État du réseau déterministe et seuils dynamiques** — fonction du journal (taux de litige /
   vérificateurs actifs, etc.), régulation clamp + sigmoïde en table, lissage temporel, bornes
   codées en dur par catégorie.
6. **Tirage probabiliste des claims** — `hash(graine_slot ‖ claim_id) < p(score)`, `p` croissant avec
   l'attente (liveness), intégration dans le flux de claim actuel (ADR 0005) sans tiers tireur.
7. **Quotas et probation** — quota de claims actifs dérivé du diagramme (quantile bas) une fois le
   seuil de confiance atteint, caution remboursable au-delà du premier claim, probation à un claim
   pour un agent neuf, intégration avec `max_parallel` existant.
8. **Ciblage déterministe des fragments** — ordre de préférence par worker
   (`hash(clé ‖ job ‖ fragment)`) et balayage par rang côté nœud (`sil-node`).
9. **Bandit d'exploration (démarrage)** — échantillonnage de Thompson pour proposer, de temps en
   temps, des claims hors du profil appris d'un agent neuf ou peu spécialisé.
10. **Retrait de l'usage attributif de `sil-reputation` v1** — l'ancien score scalaire local cesse de
    peser sur toute attribution ; il peut rester comme opinion d'affichage seulement.

## Issues associées

Voir le dépôt GitHub : une issue par phase ci-dessus, préfixée `reputation:`, référençant cet ADR.
