# Journal (`sil-ledger`)

## Modèle

Un **job** commence par un `SignedJobSpec` : le requester signe la description, le comité
de coordinateurs (1 à 7 clés) et le seuil `threshold`, **strictement supérieur à la moitié**
du comité. `job_id = SHA-256(domaine JobId, spec canonique)`.

Chaque auteur a une **chaîne par job** : `sequence` (1, 2, 3…) et `previous` (hash de son
événement précédent). Les jobs sont indépendants : rattrapage, quotas et stockage sont par job.
Le premier événement du requester est `JobPublished` (séquence 1).

## Événements

| Événement | Émetteur | Effet |
|---|---|---|
| `JobPublished` | requester | crée le job |
| `Claim` | tout pair | offre de calculer (fragment, tentative) |
| `Accept` | comité (vote) | retient un claim |
| `Renew` / `Release` | claimant | prolonge le bail / abandonne |
| `Submit` | claimant | publie le hash du résultat |
| `VerifyOffer` | tout pair sauf claimant | offre de vérifier |
| `VerifierAccept` | comité (vote) | retient un vérificateur |
| `Attest` | vérificateur retenu | verdict signé (approbation, ou rejet avec preuve) |
| `Finalize` | comité (vote) | finalise, avec un certificat d'approbations |
| `Cancel` | requester | annule le job |

## Décisions par vote

Une décision (claim retenu, vérificateur retenu, finalisation) existe quand `threshold`
membres **distincts** du comité ont émis le même vote. Seul le **premier** vote de chaque
membre sur un emplacement unique compte. Comme `threshold > n/2`, deux décisions
contradictoires sur un même emplacement sont impossibles. Avec un comité de 1 et un seuil de 1,
on retrouve le publisher unique de V0, mais c'est un choix explicite du requester.

L'heure d'acceptation est le `threshold`-ième plus petit `issued_at_ms` des votes décisifs :
elle se déduit des événements, sans horloge partagée.

## Vérification

- `verifications_required` (V) approbations de vérificateurs retenus par le comité,
  distincts du claimant, font passer à `Verified`.
- V rejets font passer à `Rejected` : un vérificateur isolé ne peut pas tuer un résultat
  (V0 : un seul rejet suffisait).
- V approbations **et** V rejets : `Disputed` ; le comité peut retenir jusqu'à `2V-1`
  vérificateurs, et `Finalize` reste impossible tant que V rejets existent.
- Un rejet porte `evidence` (hash de preuve) ; une approbation n'en porte pas.

## Finalisation (ADR 0002)

`Finalize` embarque un `Certificate`. Il est contrôlé deux fois :
1. **À l'insertion** : signatures de toutes les attestations, au moins V distinctes.
2. **À la dérivation** : au moins V approbateurs du certificat sont des vérificateurs
   que le comité a retenus et qui ont réellement approuvé ce résultat ; moins de V rejets.

Un comité complice ne peut donc pas finaliser avec un certificat fabriqué.

## État dérivé

`Ledger::job_state` / `task_state` sont des **fonctions pures** de l'ensemble d'événements
et de `now_ms` passé en argument. L'ordre d'arrivée n'a aucun effet (testé). Les statuts
d'une tâche : `Open`, `Claimed`, `LeaseExpired`, `Released`, `Completed`, `Verified`,
`Disputed`, `Rejected`, `Failed`, `Finalized`, `Cancelled`, `Expired`.
Un résultat publié en retard compte tant qu'aucune tentative suivante n'a été retenue.

Le coût d'une dérivation est borné par les quotas d'**un seul job** (V0 : tout le journal).
Aucun cache pour l'instant ; à mesurer en phase 6 avant d'en ajouter un.

## Réception, ordre et rattrapage

`Ledger::receive` vérifie dans l'ordre : signature, taille, forme, job connu (sinon
`UnknownJob`), autorisation de l'émetteur, puis la chaîne :
- suite directe : inséré, puis les événements en attente qu'il débloque ;
- en avance (≤ `max_sequence_gap`) : mis en attente, borné par auteur ;
- même séquence mais autre contenu : **équivocation**, l'auteur est ignoré pour ce job ;
- déjà connu : sans effet.

`Ledger::gaps` liste ce qu'il faut demander à un pair ; `events_after` répond (borné).
Chaque job garde aussi son **ordre d'acceptation** : `events_since(job, curseur, max)` le parcourt
par pages, ce qui sert le rattrapage par curseur (`gossip.md`).

## Quotas (`sil-quota::LedgerLimits`)

Jobs total et par requester, événements par job et par (auteur, job), octets par job,
taille d'un événement, événements en attente, écart de séquence. Tout dépassement est une
erreur typée. Valeurs par défaut dans `limits.rs`.

## Limites connues

- Un job publié par un requester quelconque est admis : la protection économique est le
  trait `Admission` (escrow vérifié en phase 5). Sans lui, seuls les quotas protègent.
- Un auteur qui équivoque n'est ignoré que par les pairs qui ont vu les deux versions.
- Pas de rotation de comité ni de coordinateur de secours : la liveness exige `threshold`
  membres joignables (en partition, l'attente est sûre, jamais de finalisation forcée).
