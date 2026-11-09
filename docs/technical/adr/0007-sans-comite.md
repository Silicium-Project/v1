# ADR 0007 : suppression du comité

**Statut :** accepté (demande du 8 novembre 2026, après ADR 0005 et 0006).

## Contexte

ADR 0001 posait un comité fixe de coordinateurs par job (`threshold` sur `n`). ADR 0005 a retiré
les votes, mais le comité est resté : dans la spec du job, dans les votes `SoloAccept` du mode solo
(ADR 0004), dans les signatures de règlement lues par la chaîne, et dans la pénalité du comité
(ADR 0006). Un job peut bloquer faute de membres du comité en ligne (job `3e0763…`, 8 novembre).

V0 n'a pas de comité. Les vérificateurs s'engagent eux-mêmes, avec leur clé et leur réputation.

## Décision

1. **Pas de comité ni de seuil.** `JobSpec` perd `committee` et `threshold`. Le texte signé du job
   perd les lignes `Committee:` et `Threshold:`.
2. **Pas de mode solo.** `SoloAccept` est supprimé. Un claimant ne vérifie jamais son résultat. Il
   faut donc au moins `k + 1` nœuds vérificateurs pour un job à `k` vérifications.
3. **Vérification ouverte** (ADR 0005, règle 4) : les `k` premières offres valides, d'auteurs
   distincts, hors claimant.
4. **Finalisation** : certificat de `k` approbations de vérificateurs distincts (inchangé). Tout
   nœud peut le publier.
5. **Autorisation de règlement = signatures des approbateurs.** Chaque approbateur signe, avec
   `Domain::Settlement`, le message `job ‖ fragment ‖ tentative ‖ résultat ‖ claimant`. Il le fait
   quand il approuve, et la signature est portée par son `Attestation`. Le message ne contient pas
   la liste des approbateurs : elle n'est connue qu'à la finalisation. La chaîne exige que chaque
   approbateur ait signé, que chacun ait un score de réputation ≥ 2 500, et qu'ils soient `k`,
   distincts, et différents du claimant.
6. **Pénalités** : supprimées par ADR 0007, rétablies par le requester (ADR 0008). Le score n'augmente qu'à
   chaque règlement approuvé (+100). **Limite corrigée par ADR 0008 :** sans pénalité, un vérificateur ne perd
   jamais sa réputation. Le gate protège seulement contre les clés non enregistrées. Un mécanisme
   de contestation par preuve reste à faire.
7. **Horloge de la dérivation** (`sil-node`, réputation dérivée) : `as_of` = plus récente date
   d'événement du job. Plus de comité pour la fixer. Cette réputation reste affichée sans peser sur
   les décisions.
8. **Protocole** : nouveau marqueur `adr-0007`. Les journaux d'avant sont mis de côté au démarrage
   (mécanisme de `node_process.rs`).

## Conséquences

- Les anciens jobs (spec avec comité) ne peuvent pas être repris : ils sont abandonnés.
- L'API perd `SIL_API_COMMITTEE` et `GET /v1/network/committee`.
- Le VPS doit être redéployé, et l'installeur republié (non fait ici : décision à prendre).
- `sil-coordinator` est supprimé. Sa seule fonction utile, bâtir `Finalize`, passe dans `sil-node`.
- Le rôle `coordinator` du nœud ne désigne plus un comité. Il est retiré de la configuration.

## Alternatives écartées

- **Garder un comité pour le règlement seulement** : la demande était de le retirer entièrement.
- **Vote on-chain par vérificateur (V0)** : même effet, plus de transactions par vérificateur et une
  récompense par vote à définir. Reporté.
- **Penalty signée par les pairs** : pas d'autorité claire ; à concevoir avec la contestation.
