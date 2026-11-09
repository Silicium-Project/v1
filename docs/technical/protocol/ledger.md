# Journal (`sil-ledger`)

## Modèle

Un **job** commence par un `SignedJobSpec` : le requester signe la description (sans comité ni seuil, ADR 0007).
`job_id = SHA-256(domaine JobId, spec canonique)`.

Chaque auteur a une **chaîne par job** : `sequence` (1, 2, 3…) et `previous` (hash de son
événement précédent). Les jobs sont indépendants : rattrapage, quotas et stockage sont par job.
Le premier événement du requester est `JobPublished` (séquence 1).

## Événements

| Événement | Émetteur | Effet |
|---|---|---|
| `JobPublished` | requester | crée le job |
| `Claim` | tout pair | offre de calculer (fragment, tentative) |
| `Renew` / `Release` | claimant | prolonge le bail / abandonne |
| `Submit` | claimant | publie le hash du résultat |
| `VerifyOffer` | tout pair sauf claimant | offre de vérifier |
| `Attest` | vérificateur retenu | verdict signé (approbation, ou rejet avec preuve) |
| `Finalize` | tout pair | certifie un résultat : désigne son claim, son `Submit`, les offres des vérificateurs qui approuvent, avec le certificat (ADR 0005, phases 3-4) |
| `Cancel` | requester | annule le job |

## Décisions sans vote

Aucun vote de coordinateur (ADR 0005, ADR 0007). Le claim retenu est le premier claim valide dans
l'ordre total `(horodatage, identifiant)`. Les vérificateurs retenus sont les `k` premières offres valides
d'auteurs distincts du claimant. Chaque nœud dérive le même résultat à partir du même journal.

## Vérification

- `verifications_required` (V) approbations de vérificateurs retenus (premières offres valides, ADR 0005),
  distincts du claimant, font passer à `Verified`.
- V rejets font passer à `Rejected` : un vérificateur isolé ne peut pas tuer un résultat
  (V0 : un seul rejet suffisait).
- V approbations **et** V rejets : `Disputed` ; les vérificateurs retenus peuvent aller jusqu'à `2V-1`
  vérificateurs. Un rejet postérieur à une certification ouvre un litige, il ne défait pas la
  finalisation (phase 3 : sinon la validité ne serait pas monotone).
- Un rejet porte `evidence` (hash de preuve) ; une approbation n'en porte pas.

## Finalisation (ADR 0002)

`Finalize` embarque un `Certificate`, ainsi que le `claim` et le `submit` qu'il certifie (ADR 0005,
phase 3). Il est contrôlé deux fois :
1. **À l'insertion** : signatures de toutes les attestations, au moins V distinctes.
2. **À la dérivation** : le claim et le `Submit` désignés existent, sont du même auteur, et le
   `Submit` tombe dans le bail du claim ; au moins V approbateurs du certificat sont des
   vérificateurs retenus pour ce claimant, qui ont approuvé ce résultat.

La validité ne lit que les événements désignés : elle ne dépend ni du vainqueur de la course aux
claims, ni des rejets postérieurs. Un claim antérieur reçu tard ne défait donc pas une
finalisation. Entre plusieurs `Finalize` valides, le premier dans l'ordre total l'emporte ; le
claimant certifié est celui du `Submit` désigné.

Un ensemble de vérificateurs complices ne peut donc pas finaliser avec un certificat fabriqué sans signatures valides.

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
- Pas de comité ni de coordinateur : la vivacité exige un worker et `V` vérificateurs éligibles en ligne.
  membres joignables (en partition, l'attente est sûre, jamais de finalisation forcée).
