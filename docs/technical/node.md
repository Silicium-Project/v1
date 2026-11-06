# Le nœud (`sil-node`)

`sil-node` **assemble** ; il ne contient pas de logique métier. Il démarre dans cet ordre :
rejeu du journal depuis le disque → magasin de contenu → outbox de règlement → réseau →
acteurs → API de contrôle.

## Acteurs

Chaque acteur est une tâche tokio indépendante. Ils ne s'appellent pas entre eux : ils lisent
et écrivent l'état partagé (`Shared`, verrous brefs, jamais tenus à travers un `await`) et
communiquent par le réseau.

| Acteur | Période | Rôle |
|---|---|---|
| `gossip_loop` | événements | décode les messages validés ; comble les trous (requête point à point à 4 pairs au plus) ; toutes les 8 s rediffuse la dernière entrée de nos chaînes et, pour un membre du comité, l'annonce du job |
| `coordinator_loop` | 500 ms | vote claims, vérificateurs, finalisation (`sil-coordinator`) ; signe les règlements des fragments finalisés et diffuse la signature |
| `worker_loop` | 500 ms | réclame les fragments ouverts **si l'escrow est sain** (`check_escrow`, cache de 5 s), lance ceux qui lui sont attribués |
| `verifier_loop` | 500 ms | propose de vérifier ; recalcule selon la politique du workload ; publie le verdict signé |
| `settlement_loop` | 1 s | fait avancer l'outbox et la sauvegarde |
| `presence_loop` | 2 s | publie la présence (rôles, charge) si elle a changé ou après 30 s |

Rôles (`Roles`) : `coordinator`, `worker`, `verifier`, combinables. Un membre du comité peut aussi
être worker ; il ne peut pas vérifier son propre résultat (règle du journal).

## Flux d'un job

1. Le requester signe `JobPublished` et l'envoie à l'API d'un nœud (`POST /v1/events`) ; le contenu
   d'entrée est envoyé avant (`PUT /v1/content`).
2. Le nœud ingère, persiste, diffuse sur `announce` et sur le fragment de topic du job.
3. Un worker vérifie l'escrow, émet `Claim` ; le comité vote `Accept` (fenêtre de 1 s, puis
   alignement sur le premier vote visible).
4. Le worker télécharge l'entrée par morceaux vérifiés, calcule dans un thread dédié (délai
   `limits().wall_ms`), range le résultat (`ResultBlob` : octets + milestones) dans le magasin et
   publie `Submit` avec son `ContentId`.
5. Un vérificateur propose, est retenu, télécharge le résultat, refait le calcul ou un échantillon
   (graine signée par lui), publie `Attest`.
6. Le comité vote `Finalize` avec le certificat ; chaque membre signe le règlement ; l'outbox de
   chaque coordinateur soumet à la chaîne dès `threshold` signatures (idempotent).

## Ce qui est testé de bout en bout

`crates/sil-node/src/tests/` démarre de **vrais nœuds libp2p** en boucle locale, pilotés par la
**vraie API HTTP**, contre `FakeChain` :

| Scénario | Vérifie |
|---|---|
| nominal avec pannes de chaîne | 4 nœuds, 2 fragments ; 4 lectures d'escrow en `429` et 3 envois en erreur injectés ; job finalisé partout ; résultats égaux à un calcul indépendant ; **220 payés exactement** ; les 3 outbox convergent vers `Confirmed` |
| worker menteur | le menteur est le seul à réclamer la tentative 1, envoie un faux hash ; le vérificateur le rejette ; l'honnête réclame la tentative 2 et est payé ; **le menteur n'est jamais payé** |
| perte d'un membre du comité | 1 membre sur 3 arrêté avant le job : le seuil (2) reste atteignable, le job finit et est payé |
| présence | deux nœuds voient chacun les rôles de l'autre via `/v1/peers` |
| bail | un calcul de 3 s avec un bail de 1,5 s garde sa tentative (`Renew`) |
| arrivée tardive | un nœud lancé après la fin du job reconstruit tout l'état par rattrapage |
| API | jetons, scopes, contenu envoyé comme octets, événement invalide refusé |
| redémarrage | le journal est rejoué depuis le disque |

Ces tests ont révélé deux défauts réels, corrigés :

1. **Votes irrévocables divisés** : deux coordinateurs vivants votaient chacun pour *leur* claim avant
   d'avoir vu l'autre ; avec 2 membres sur 3 il ne pouvait plus jamais y avoir 2 voix. Corrigé par une
   fenêtre d'observation et l'alignement sur le premier vote visible (voir `coordination.md`).
2. **Téléchargements concurrents du même contenu** : deux fragments partageant l'entrée écrivaient le
   même fichier partiel ; le second échouait et le worker relâchait sa tentative. Corrigé : le magasin
   refuse un second téléchargement (`InProgress`) et le nœud en sérialise un par contenu.

## Mécanismes ajoutés en fin de phase 6

- **Renouvellement du bail** : pendant le calcul, le worker émet `Renew` tous les tiers de bail
  (10 au plus). Sans cela, le comité aurait accepté un autre worker pour un travail en cours.
- **Rattrapage par curseur** : toutes les 2 s, pour chaque job connu et deux pairs au plus, le nœud
  demande « les événements du journal du pair depuis ma position » (pages de 64). Un nœud arrivé
  tard reconstruit ainsi l'historique complet, y compris les auteurs qu'il ne connaissait pas.
  Un pair à jour n'est redemandé qu'après 15 s.
- **Chef de tour** : les membres du comité qui ne sont pas le chef attendent son vote (3 s au plus
  après la fenêtre d'observation) avant de voter ; voir `protocol/coordination.md`.
- **Présence** : toutes les 2 s le nœud évalue sa présence (rôles, charge = tâches en cours,
  workloads) et ne la publie que si elle change ou après 30 s de silence. Les présences reçues
  (table bornée à 1024 pairs, oubli après 3 min) alimentent `GET /v1/peers`.
- **Abonnements ciblés** : au démarrage le nœud n'écoute que `presence` et `announce` ; il
  s'abonne au fragment de topic d'un job quand il apprend ce job (annonce, API ou rejeu du
  journal). Les événements manqués avant l'abonnement viennent du rattrapage par curseur.
- **Réputation alimentée** : un membre du comité qui fait confiance au requester (`[trust]
  requesters`) compte les tentatives finalisées comme succès, les remplacées ou échouées comme
  échecs, une fois chacune.

## Limites connues

- **Chaîne réelle seulement avec `chain/node`** : le binaire `sil-node` (`silicium-node`) n'a pas de
  chaîne et refuse une section `[chain]`. Le binaire `chain/node` (même démarrage, adaptateur
  Solana) règle les jobs sur l'escrow localnet ; les tests sur réseau public restent à faire.
  Les workers réclament un fragment seulement si l'escrow est sain (`check_escrow`).
- **Votes irrévocables** : le chef de tour réduit fortement les divisions 1–1 mais ne les exclut
  pas (chef muet puis revenu, vues différentes). Pas de rotation de comité.
- Les **clés restent dans le processus du nœud** (séparation en processus signataire non faite).
- Sans `[trust] requesters`, la réputation reste neutre.
- Le téléchargement s'appuie sur tous les pairs connectés, sans choix par latence ni charge.
- Pas de traversée de NAT (voir `gossip.md`).
- L'API de contrôle est en HTTP clair (boucle locale par défaut) ; pas de TLS.
