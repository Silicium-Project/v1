# ADR 0012 — Vote du quorum, pénalité de la minorité et partage de la caution

**Statut** : implémentée côté chaîne (`sil-chain-core`, programme, adaptateur). La preuve est le
verdict déjà signé par chaque vérificateur (`Domain::Attestation`), reconstruit on-chain dans un
format binaire fixe (`attestation_frame`, identique à `sil_attest::verdict_message`). Contraire =
approuver un autre résultat, ou rejeter le résultat réglé avec sa preuve. Le nœud n'a rien de
plus à produire.

Envoi : le nœud détecte les verdicts contraires des vérificateurs retenus (`penalty_claims`, sur la
tentative finalisée), les met en file (`PenaltyBox`, sil-settlement) quand le règlement est confirmé,
et les envoie via `Chain::penalize`. Seul un nœud qui paie les frais envoie. La file n'est pas
sauvegardée : elle est reconstruite depuis le journal à chaque tour. Les verdicts tolérance
(embedding) sont couverts : ils ont leur propre trame (`tolerance_frame`, domaine
`sil/v1/attestation-tolerance`), et seuls ceux dont le modèle et le runtime sont ceux du job comptent. Complète l'ADR 0010 et remplace la pénalité signée par le
requester (ADR 0008). Aujourd'hui le code confisque la caution sur la seule signature du requester et
la verse au requester.

## Flux

1. Le requester soumet une tâche sur le site. Il ouvre l'escrow et dépose les fonds.
2. Les workers récupèrent les fragments, calculent et publient leurs résultats.
3. Un quorum de `k` vérificateurs vérifie chaque fragment et vote. Les votes sont des événements
   signés du journal ; le quorum reste hors chaîne.
4. Le résultat retenu est celui de la majorité. Les approbateurs de la majorité signent le règlement,
   qui est on-chain.
5. Un vérificateur qui a voté l'inverse de la majorité est considéré malveillant. Il est puni.
6. Si aucun vérificateur ne s'est trompé, personne ne touche la caution : chaque vérificateur la
   récupère à l'échéance (ADR 0010 : 20 règlements ou 24 h après son dernier vote).

## Décision proposée

### 1. Pénalité de la minorité

Un vérificateur qui a voté contre la majorité sur un fragment perd de la réputation (ADR 0008) et la
caution de ce fragment : la part de sa réserve égale à 50 % de la récompense de vérificateur de ce
fragment, plafonnée à sa réserve actuelle (ADR 0010).

### 2. Partage de la caution confisquée

- **70 %** au requester.
- **30 %** partagés à parts égales entre les membres du quorum qui ont voté avec la majorité.
- Sans vérificateur malveillant, rien n'est confisqué.

### 3. Preuve

Le quorum étant hors chaîne, la preuve vient de la transaction de pénalité :

- **Majorité** : la liste des approbateurs réglés est déjà on-chain. Elle donne les bénéficiaires des
  30 %, sans rien ajouter.
- **Minorité** : la transaction joint la signature du vote du fautif sur un résultat différent du
  résultat réglé, pour le même job, fragment et tentative. Le programme la vérifie avec le
  précompilé Ed25519, comme pour un règlement. Le fautif a signé lui-même ce vote : personne ne peut
  le fabriquer à sa place.
- **Qui décide** : le quorum. La pénalité découle de la signature du vote contraire à la majorité, pas
  d'une décision du requester. Le requester n'a aucun pouvoir de pénalité. La transaction peut être
  envoyée par n'importe qui, mais le programme n'applique la pénalité que si la preuve de vote
  contraire est valide. Un seul envoi par (fragment, vérificateur).

### 4. Taille du quorum `k`

`k` est toujours impair. Ainsi une majorité stricte existe toujours : elle vaut `(k+1)/2` votes, et
il n'y a jamais d'égalité.

`k` dépend de la valeur de la tâche et du nombre de vérificateurs disponibles. Règle par défaut :
`k = plus grand impair ≤ min(max(k_valeur, k_min), k_dispo, 15)`, c'est-à-dire la plus grande
exigence, plafonnée par les vérificateurs disponibles, et ramenée à un nombre impair. Le plafond est
15 : 16 est pair. Variante à trancher : pondérer par la réputation.

## Conséquences

- **Pas de preuve de sybil à fournir sur la majorité** : seuls des vérificateurs ayant signé le
  règlement reçoivent les 30 %. Un requester ne peut pas fabriquer le vote d'un tiers.
- **Format** : nouveau message signé de vote (domaine propre, à définir) et nouvelle forme de
  `Penalize` (signature du vote fautif jointe). Le compte d'escrow ne change pas.
- **Fin du pouvoir unilatéral du requester** : il ne peut plus confisquer sans vote contraire signé.
- **Le requester n'est pénalisé en rien** s'il n'y a pas de fautif : la caution revient au
  vérificateur.
- Déploiement incompatible, comme l'ADR 0010.

## Points ouverts

- Majorité : `(k+1)/2` votes identiques (acquis, k impair). Plus d'égalité à traiter.
- Un vérificateur qui ne vote pas du tout n'est pas visé.
- Un vote signé qui n'a pas été publié dans le journal avant le règlement : à quelle condition il
  compte comme « contre ».
- Message signé du vote et domaine de signature.
- Valeur de `k_min` et fonction `k_valeur`.
- Fenêtre de dépôt de la preuve : au plus tard avant la libération de la réserve (ADR 0010).
