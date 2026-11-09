> **Remplacé par ADR 0007 :** le vote `SoloAccept` et le mode solo sont supprimés.

# ADR 0004 : vérification solo (un nœud seul vérifie son propre calcul)

## Contexte

Un résultat doit être vérifié par un vérificateur **distinct** du claimant (`verdicts.rs` :
`the_claimant_cannot_be_its_own_verifier`, et `security.rs` : le claimant ne gonfle pas le certificat
avec sa propre approbation). Sur un réseau d'un seul nœud, rien ne peut donc finaliser un calcul.
Le besoin : pouvoir tester et faire tourner le réseau seul, sans affaiblir le cas normal.

## Décision

Un nouveau vote du comité, `SoloAccept { fragment, attempt, offer }`, retient **l'offre de
vérification du claimant lui-même**. Il n'est valable que si l'offre est un `VerifyOffer` de
la tentative **émis par le claimant**. Le résultat porte le drapeau `solo` (`VerifierState.solo`).

Chemin, pour une tentative publiée à `t0` :

1. le claimant publie son `Submit` ;
2. le claimant attend `solo_wait_ms` ; si aucun autre vérificateur ne s'est proposé
   (`open_verifications` vide, hors claimant), il émet son propre `VerifyOffer` ;
3. le comité, à `t0 + solo_wait_ms`, sans autre offre ni vérificateur retenu, vote `SoloAccept`
   sur cette offre ;
4. le claimant vérifie son calcul et publie son `Attest`, comme un vérificateur ordinaire.

Le cas normal ne change pas : `VerifierAccept` ne retient toujours que des offres d'autres
auteurs que le claimant. `SoloAccept` ne retient que des offres du claimant.

## Garde-fous

- **Désactivé par défaut** (`[testnet] solo_verify = false`).
- **Refusé avec `[chain]`** : un résultat qui rapporte de l'argent doit être vérifié par un tiers.
- **Visible** : `solo: true` dans l'état du job. Une interface peut afficher « vérifié par le seul
  nœud » et le rapport de réputation peut en tenir compte.
- **Décidé par le comité** : le claimant ne peut pas se retenir lui-même. Un `SoloAccept` émis
  par un non-membre est refusé à l'insertion (`job.rs`).
- **Seulement si personne d'autre ne s'est proposé** : tant qu'un autre vérificateur existe, le cas
  normal s'applique.

## Conséquences

- Un résultat **solo** n'est pas un résultat contrôlé : le claimant juge son propre calcul. Sur un
  réseau ouvert, un worker malhonnête peut faire passer un faux résultat s'il est seul ou si les
  autres vérificateurs restent muets pendant `solo_wait_ms`. C'est acceptable seulement en test.
- Le comité seul décide de la fenêtre. Un comité qui triche peut retenir un solo prématurément :
  même confiance que pour les autres votes du comité.
- Le format des événements existants ne change pas : `SoloAccept` est un variant ajouté.

## Alternatives écartées

- **Supprimer la règle « pas de self-vérification »** : le claimant approuverait toujours son
  propre calcul, sans trace. Refusé.
- **Vérifier « seul sur le réseau » par la présence (gossip)** : non déterministe, non vérifiable
  par les autres nœuds. Refusé : la décision reste dans le journal signé.
- **Un nœud sans vérification (accepter sans contrôle)** : pire, le drapeau `solo` le rend visible.

## Vérification

- `sil-ledger` : un `SoloAccept` sur sa propre offre retient le claimant (`solo: true`) ; un
  `SoloAccept` sur l'offre d'un autre est ignoré.
- `sil-coordinator` : pas de vote avant le délai ; vote après le délai ; pas de solo si un autre
  vérificateur s'est proposé ; pas de solo sans `with_solo`.
- `sil-node` : un nœud seul finalise un job (`a_lone_node_verifies_its_own_result_and_finalizes`).
