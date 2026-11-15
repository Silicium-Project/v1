# ADR 0009 — Preuve d'horloge pour l'ordre des claims

**Statut** : implémentée. `EventBody::Claim` porte `anchor_slot` ; l'ordre des claims est
`(anchor_slot, issued_at_ms, id)` ; à la réception en direct, un claim dont le slot est hors de la
fenêtre locale est refusé. Choix d'implémentation : tolérance de 2 slots au-dessus du slot local
(`CLAIM_LAG_TOLERANCE_SLOTS`), pour le retard de l'RPC local ; les claims hors direct (rattrapage)
passent sans borne, comme les autres événements historiques.

Limite assumée : la borne de réception dépend du slot lu par le nœud. Deux nœuds dont l'RPC diffère
de plus de la fenêtre peuvent accepter des ensembles de claims différents. Le tri, lui, reste
déterministe : il ne lit que le slot signé dans l'événement.

## Contexte

Un `claim` gagne une tentative selon l'ordre total `(issued_at_ms, id)` (ADR 0008 et `sil-ledger`,
`state/attempt.rs`). Or `issued_at_ms` est déclaré par l'auteur. Un worker peut antidater son claim
et gagner la course.

Mesure actuelle (non suffisante) : `sil-node/src/ingest.rs` refuse, à la réception en direct
(gossip ou API), un événement plus vieux que `MAX_LIVE_AGE_MS` (60 s) ou plus récent que
`MAX_SKEW_MS` (30 s). Un antidatage de jusqu'à 60 s reste possible. La borne dépend de l'horloge
locale de chaque nœud, donc de sa synchronisation.

## Décision proposée

Ancrer chaque `claim` à une horloge externe et vérifiable : le slot Solana récent vu par le worker.

1. `EventBody::Claim` porte un champ `anchor_slot: u64`, en plus de `fragment` et `attempt`.
2. Le réducteur ordonne les claims d'une tentative par `(anchor_slot, issued_at_ms, id)`.
3. À la réception en direct, un nœud vérifie `anchor_slot` dans une fenêtre `[slot_rpc - W, slot_rpc]`
   avec `W` de l'ordre de 150 slots (environ une minute). Hors fenêtre, l'événement est refusé.
4. Un worker ne peut donc plus antidater son claim au-delà de `W` slots, quelle que soit l'horloge
   locale.

## Conséquences

- **Format** : changement du type `Claim`, donc de l'encodage canonique. Les vecteurs
  (`contracts/vectors/`) et les tests `sil-ledger` (~20 constructeurs) sont à mettre à jour.
- **Dépendance RPC** : tout nœud qui valide un claim doit avoir un RPC Solana. Un nœud sans
  `[chain]` ne peut plus valider les claims. Aujourd'hui, un worker sans chaîne ne réclame déjà rien.
- **Déploiement** : changement incompatible. Tous les nœuds (VPS, postes desktop, nœuds locaux)
  doivent passer à la nouvelle version en même temps. Les journaux existants sont mis de côté
  (mécanisme `adr-0007` du desktop).
- **Limite** : deux claims dans le même slot (≈ 400 ms) restent ordonnés par `issued_at_ms`. La
  fenêtre est réduite à quelques centaines de millisecondes, pas à zéro.
- **Dépendance au RPC** : si le RPC est en panne ou ment, la fenêtre devient fausse. Il faut
  plusieurs RPC ou un contrôle croisé (hypothèse à trancher).

## Alternatives

- **Horloge locale bornée** (état actuel, 60 s) : simple, mais dépend de la synchronisation locale.
- **Ordre par réception** au témoin : pas déterministe entre nœuds, rejeté.
- **Preuve causale** (référence au dernier événement vu) : ne borne pas l'antidatage sans fenêtre.
- **Horloge de consensus léger** : complexité disproportionnée pour le testnet.

## Correctif de revue : règle de journal sur la fraîcheur

La borne de réception seule ne suffit pas : un claim antidaté reçu par rattrapage (sync) n'y est
pas soumis, et l'ordre `(anchor_slot, …)` le placerait devant un claim frais, en redéfinissant un
gagnant déjà accepté. Règle ajoutée dans le journal (`sil-ledger`, `state/attempt.rs`) : pour une
tentative, un claim ne vaut que si `anchor_slot + CLAIM_WINDOW_SLOTS ≥ max(anchor_slot des claims de
cette tentative)`. Elle ne dépend d'aucune horloge locale. Conséquence assumée : un claim frais qui
arrive après un claim plus ancien peut invalider ce dernier, car le plus récent fixe la référence.
