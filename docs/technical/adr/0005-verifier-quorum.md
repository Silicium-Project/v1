# ADR 0005 : quorum de vérificateurs, sans comité de coordinateurs

## Contexte

Aujourd'hui un job est décidé par un comité fixe de trois coordinateurs (ADR 0001) : ce sont eux
qui votent les claims (`Accept`), les vérificateurs (`VerifierAccept`) et la finalisation
(`Finalize`). Trois limites en découlent :

- **Vivacité** : sans deux coordinateurs en ligne, aucun fragment n'est attribué (constaté sur le
  devnet, job `Active` sans tentative).
- **Centralisation** : les trois clés sont choisies par l'opérateur (`SIL_API_COMMITTEE`).
- **Réputation locale** : chaque node calcule la réputation des workers et des vérificateurs à
  partir de ses propres observations. Deux nodes peuvent classer différemment les offres et ne
  pas s'accorder sur le vérificateur retenu.

Le besoin : un réseau où un worker et un vérificateur suffisent à faire avancer un calcul, sans
organe de vote, et où tous les nodes décident de la même façon à partir du même journal.

## Décision

Plus aucun vote de coordinateur. Chaque décision est une **règle déterministe sur l'état du
journal**. Tous les nodes qui ont le même journal obtiennent le même résultat. Un nœud ne « vote »
plus : il publie un événement signé (claim, offre de vérification, verdict), et les autres
événements sont lus, pas votés.

### Règles

1. **Ordre total des événements.** Tri par `(issued_at_ms, identifiant de l'événement)`. Un
   événement daté dans le futur de plus de `MAX_SKEW_MS` par rapport au nœud qui le reçoit est
   rejeté (voir questions ouvertes).
2. **Claim.** Pour un (fragment, tentative), le gagnant est le premier `Claim` valide dans l'ordre
   total. Un `Claim` est valide si le claimant a le droit de réclamer (escrow sain, quand la
   chaîne est active) et si le fragment n'a pas déjà une tentative en cours.
3. **Bail.** Une tentative sans `Submit` ni `Renew` dans son bail (`lease_ms`) expire. Le
   fragment repasse en `Open` et une nouvelle tentative peut être réclamée.
4. **Vérificateurs.** Le spec du job fixe `k` (`verifications_required`). Les premières `k` offres
   de vérification valides, d'auteurs distincts du claimant, sont retenues dans l'ordre total.
   Un vérificateur qui a une réputation de vérificateur faible peut être classé après les autres
   offres, comme aujourd'hui, à condition que ce classement soit calculé sur le journal (règle 5).
5. **Réputation dérivée du journal.** Le score d'un auteur (worker et vérificateur) est calculé
   à partir des événements seuls : tentatives gagnées ou perdues, verdicts justes ou faux. Il n'y a
   plus d'observation locale. Cela reprend le principe de la V0 (`author_records`).
6. **Finalisation.** Un fragment est finalisé quand `k` `Attest` approuvent la tentative gagnante,
   sans rejet valide. N'importe quel nœud peut publier le `Finalize`, avec les `k` attestations
   comme certificat (ADR 0002). Le `Finalize` n'a plus besoin d'être signé par un coordinateur.
7. **Pas d'auto-validation.** Le claimant ne compte jamais parmi ses propres vérificateurs.

## Conséquences

- **Changement de consensus.** Tous les nodes doivent passer à la nouvelle règle ensemble : un
  node ancien et un node nouveau décideraient différemment. Le déploiement est coordonné (VPS,
  sims, installeurs), comme pour le classement des vérificateurs.
- **Rôle coordinateur retiré** du protocole. Le code de vote reste compilable pendant la
  transition, mais n'est plus appelé.
- **Sécurité** : avec `k = 1`, un seul vérificateur décide (acceptable sur devnet, pas en
  production). Avec `k ≥ 2`, il faut deux vérificateurs indépendants, donc au moins trois
  machines distinctes (worker, deux vérificateurs).
- **Journal existant** : le devnet peut repartir d'un nouveau genesis. Pas de migration des
  décisions passées.

## Questions ouvertes

- **Dérive d'horloge.** L'ordre total dépend de `issued_at_ms`, donc de l'horloge des pairs. Un pair
  qui avance son horloge peut prendre un claim. Option : tolérance `MAX_SKEW_MS` (30 s) et
  rejet des événements hors fenêtre. À trancher avant la phase 2.
  *Réputation dérivée (phase 1b)* : le temps de jugement est l'horloge du comité du job
  (`derived_reputation/clock.rs`, seuil-ième plus grande date de tête), pas celle du nœud. Un membre
  seul ne fait ni avancer ni figer un bail. Le consensus (claims, baux) ne lit pas encore cette
  horloge : la règle de bail reste à écrire en phase 2.
- **Valeur de `k`** par défaut (1 en test, 2 ou plus en production) et par workload.
- **Rejet par preuve.** Un vérificateur qui rejette doit publier une preuve de divergence (blocs
  divergents signés, ADR 0004). Le format reste à fixer.
- **Sybil.** Sans comité, une clé jetable peut tenter de devenir vérificateur. La réputation
  dérivée et le mode probation (tour sur trois) limitent le risque, sans l'éliminer.

## Plan

1. **Réputation dérivée du journal** : fonction pure sur l'état du journal, testée pour qu'elle
   donne le même score sur deux nodes au journal identique. Sans changement de consensus.
   *Fait* : `sil_reputation::from_observations` (phase 1a) ; `sil-node/src/derived_reputation.rs`
   relit le journal et l'expose dans le rapport worker (`reputation_derived`, phase 1b). Les
   décisions (classement des vérificateurs, comité) ne la lisent pas encore.
2. **Claim et bail sans vote** : le reducer retient le premier claim valide dans l'ordre total.
   Tests avec des événements réordonnés et un claim tardif.
   *Fait* (`sil-ledger/src/state/attempt.rs`, tests `tests/claims.rs`) :
   - un claim est valide s'il n'est pas d'un ancien claimant du fragment, et, pour la tentative
     `k > 1`, si la tentative `k-1` est libérée, rejetée, ou a un bail échu **à l'heure d'émission
     du claim** ;
   - le bail vaut `claim.issued + lease_ms`, étendu par chaque renouvellement émis avant
     l'échéance ; un `Submit` émis après l'échéance est sans effet ;
   - la date d'un événement reçu est bornée : `MAX_SKEW_MS` = 30 s au-delà de l'horloge du nœud
     (`sil-node/src/ingest.rs`). C'est le seul point où le nœud lit son horloge.
   - Gap corrigé en phase 3 (test `a_backdated_claim_cannot_undo_a_finalization`) : un claim daté
     avant celui qui a abouti, reçu après la finalisation, ne défait plus la certification.
3. **Vérificateurs et quorum `k`** : premières `k` offres retenues, finalisation sur `k`
   attestations. Tests : 1 worker + 1 vérificateur (`k = 1`), 2 vérificateurs (`k = 2`), un
   vérificateur malhonnête minoritaire.
4. **Retrait du vote** dans les boucles du node, e2e local à trois nœuds sans coordinateur.
   *Risques réglés* :
   - **Liste blanche** (remplacée par ADR 0006) : la première version ne retenait que les membres
     du comité du job (`SIL_API_COMMITTEE`). Ce comité bloquait les jobs faute de membres en ligne.
     Désormais toute offre valide peut être retenue ; la réputation on-chain est contrôlée au
     règlement.
   - **Finalité** : `Finalize` désigne les offres des vérificateurs qui approuvent (`verifiers`).
     Sa validité ne lit que ces événements : une offre plus ancienne reçue tard ne défait rien.
   - **Solo** : désactivé par défaut (`solo_wait_ms` absent). Seul vote restant avec la signature
     des règlements (phase 5).
   *Fait* : `coordinator_loop` ne vote plus ; tout nœud publie `Finalize` (`sil_coordinator::finalization`).
   Test `tests/no_coordinator.rs` (trois nœuds, aucun `Accept`/`VerifierAccept`). Reste : vote
   `SoloAccept` (ADR 0004) et signatures de règlement du comité (phase 5).
5. **Mise à jour coordonnée** du VPS, des sims et des installeurs, puis test sur le devnet.
