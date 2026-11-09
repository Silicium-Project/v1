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
| `gossip_loop` | événements | décode les messages validés ; comble les trous (requête point à point à 4 pairs au plus) ; toutes les 8 s rediffuse la dernière entrée de nos chaînes et l'annonce du job |
| `finalize_loop` | 500 ms | publie `Finalize` des résultats approuvés (`finalization.rs`) ; met en file le règlement signé par les approbateurs |
| `worker_loop` | 500 ms | réclame les fragments ouverts **si l'escrow est sain** (`check_escrow`, cache de 5 s), lance ceux qui lui sont attribués |
| `verifier_loop` | 500 ms | propose de vérifier ; recalcule selon la politique du workload ; publie le verdict signé |
| `settlement_loop` | 1 s | fait avancer l'outbox et la sauvegarde |
| `presence_loop` | 2 s | publie la présence (rôles, charge) si elle a changé ou après 30 s |

Rôles (`Roles`) : `worker`, `verifier`, combinables. Il n'y a plus de comité (ADR 0007) : tout nœud finalise.
Un claimant ne vérifie jamais son propre résultat.

## Flux d'un job

1. Le requester signe `JobPublished` et l'envoie à l'API d'un nœud (`POST /v1/events`) ; le contenu
   d'entrée est envoyé avant (`PUT /v1/content`).
2. Le nœud ingère, persiste, diffuse sur `announce` et sur le fragment de topic du job.
3. Un worker vérifie l'escrow, émet `Claim`. Depuis la phase 2 (ADR 0005), le premier claim valide
   dans l'ordre total (heure d'émission, identifiant) retient la tentative, sans vote ; les `Accept`
   n'y change rien.
4. Le worker télécharge l'entrée par morceaux vérifiés, calcule dans un thread dédié (délai
   `limits().wall_ms`), range le résultat (`ResultBlob` : octets + milestones) dans le magasin et
   publie `Submit` avec son `ContentId`.
5. Un vérificateur propose, est retenu, télécharge le résultat, refait le calcul ou un échantillon
   (graine signée par lui), publie `Attest`.
6. Tout nœud publie `Finalize` (claim, submit, certificat). Chaque approbateur a signé le règlement dans son `Attest` ; l'outbox de
   chaque nœud soumet à la chaîne dès que les `V` signatures d'approbateurs sont là (idempotent).

## Ce qui est testé de bout en bout

`crates/sil-node/src/tests/` démarre de **vrais nœuds libp2p** en boucle locale, pilotés par la
**vraie API HTTP**, contre `FakeChain` :

| Scénario | Vérifie |
|---|---|
| nominal avec pannes de chaîne | 4 nœuds, 2 fragments ; 4 lectures d'escrow en `429` et 3 envois en erreur injectés ; job finalisé partout ; résultats égaux à un calcul indépendant ; **220 payés exactement** ; les 3 outbox convergent vers `Confirmed` |
| worker menteur | le menteur est le seul à réclamer la tentative 1, envoie un faux hash ; le vérificateur le rejette ; l'honnête réclame la tentative 2 et est payé ; **le menteur n'est jamais payé** |
| perte d'un worker | un worker arrêté avant le job : les autres nœuds finissent le job et le règlement passe |
| présence | deux nœuds voient chacun les rôles de l'autre via `/v1/peers` |
| bail | un calcul de 3 s avec un bail de 1,5 s garde sa tentative (`Renew`) |
| arrivée tardive | un nœud lancé après la fin du job reconstruit tout l'état par rattrapage |
| API | jetons, scopes, contenu envoyé comme octets, événement invalide refusé |
| redémarrage | le journal est rejoué depuis le disque |
| relais (`sil-net`) | deux pairs qui ne connaissent que le relais se joignent par `p2p-circuit` et le gossip passe |

Ces tests ont révélé deux défauts réels, corrigés :

1. **Votes divisés** : supprimés avec le comité (ADR 0005, ADR 0007). Le premier claim valide l'emporte.
2. **Téléchargements concurrents du même contenu** : deux fragments partageant l'entrée écrivaient le
   même fichier partiel ; le second échouait et le worker relâchait sa tentative. Corrigé : le magasin
   refuse un second téléchargement (`InProgress`) et le nœud en sérialise un par contenu.

## Mécanismes ajoutés en fin de phase 6

- **Renouvellement du bail** : pendant le calcul, le worker émet `Renew` tous les tiers de bail
  (10 au plus). Sans cela, un autre worker pourrait réclamer un travail en cours.
- **Rattrapage par curseur** : toutes les 2 s, pour chaque job connu et deux pairs au plus, le nœud
  demande « les événements du journal du pair depuis ma position » (pages de 64). Un nœud arrivé
  tard reconstruit ainsi l'historique complet, y compris les auteurs qu'il ne connaissait pas.
  Un pair à jour n'est redemandé qu'après 15 s.
- **Pas de comité** (ADR 0007) : ni chef de tour ni rotation à gérer.
- **Présence** : toutes les 2 s le nœud évalue sa présence (rôles, charge = tâches en cours,
  workloads) et ne la publie que si elle change ou après 30 s de silence. Les présences reçues
  (table bornée à 1024 pairs, oubli après 3 min) alimentent `GET /v1/peers`.
- **Abonnements ciblés** : au démarrage le nœud n'écoute que `presence` et `announce` ; il
  s'abonne au fragment de topic d'un job quand il apprend ce job (annonce, API ou rejeu du
  journal). Les événements manqués avant l'abonnement viennent du rattrapage par curseur.
- **Réputation alimentée** : un nœud qui fait confiance au requester (`[trust] requesters`) compte les tentatives finalisées comme succès, les remplacées ou échouées comme échecs, une fois chacune.

## Limites connues

- **Chaîne réelle seulement avec `chain/node`** : le binaire `sil-node` (`silicium-node`) n'a pas de
  chaîne et refuse une section `[chain]`. Le binaire `chain/node` (même démarrage, adaptateur
  Solana) règle les jobs sur l'escrow. Testé sur devnet : un job complet (`chain/devnet-job`)
  avec le VPS et deux nœuds locaux, 2/2 fragments réglés, payé 245280 / 245280 lamports.
  Les workers réclament un fragment seulement si l'escrow est sain (`check_escrow`).
- **Pas de comité** (ADR 0007) : aucune décision de vote, donc aucune division possible. Un vérificateur qui disparaît laisse sa place à un autre.
- Les **clés restent dans le processus du nœud** (séparation en processus signataire non faite).
- Sans `[trust] requesters`, la réputation reste neutre.
- Le téléchargement s'appuie sur tous les pairs connectés, sans choix par latence ni charge.
- **NAT** : un nœud derrière NAT réserve un circuit auprès de chaque pair d'amorçage (relais) et
  devient joignable via `relais/p2p-circuit/p2p/<lui>`. Le relais (`relay_server = true`, nœud
  joignable directement) borne les circuits : 1 h et 1 Gio par circuit. DCUtR tente ensuite une
  connexion directe (candidats issus d'identify). Non validé : la perforation réelle entre deux
  NAT distincts, les tests ont tourné sur une seule machine.
- L'API de contrôle est en HTTP clair (boucle locale par défaut) ; pas de TLS.
