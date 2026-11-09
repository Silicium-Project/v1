# Règlement (`sil-chain-core`, `sil-settlement`)

## Principe

Le journal P2P décide *qui a fait quoi* ; la chaîne ne fait que **payer**, une fois, sur
autorisation des approbateurs (leurs signatures). La chaîne ne rejoue pas le calcul et ne lit pas le journal : elle vérifie
des signatures et applique les règles de l'escrow.

```
fragment Finalized (journal) ──► SettlementClaim ──► signatures des V approbateurs
        ──► Outbox ──► Chain::submit ──► programme : sil-chain-core::Escrow::settle ──► transferts
```

Le lien P2P ↔ chaîne est `job_id = SHA-256(domaine JobId, JobSpec)` : le même identifiant dans les
deux systèmes. (V0 : aucune preuve ne reliait un `TaskFinalized` à un règlement.)

## Règles de l'escrow (`sil-chain-core`, testées)

| Règle | Détail |
|---|---|
| Ouverture | signée par le requester ; dépôt ≥ Σ budget des fragments (`unités × (reward_per_unit + V × verifier_reward_per_unit) + frais`) |
| Unités | fixées par la spec signée, jamais par un coordinateur ni par le worker. `calcul/sha256` : une unité = un octet. Le fragment `i` a `len / n` octets, plus un pour les `len mod n` premiers. Entrée plus petite que `n` : refusée. `rendu/raytracer` : une unité = un pixel. Les `ceil(hauteur / 8)` blocs de 8 lignes sont répartis comme `sil-wl-render` ; une bande vaut `largeur × lignes`, la dernière coupée à la hauteur. La largeur et la hauteur sont dans la spec signée, et le worker refuse de calculer si l'en-tête de l'entrée ne les donne pas. Image avec moins de blocs que de fragments : refusée. Autre workload : refusé à l'ouverture |
| Règlement | exige la signature de **chacun** des `V` approbateurs (distincts, différents du claimant), chacun éligible on-chain (score ≥ 2 500). Un fragment au plus une fois |
| Double règlement | `AlreadySettled { same }` : `same = true` veut dire « c'est déjà fait comme voulu » |
| Remboursement | requester seul, après `échéance + 24 h` ; tout le solde non payé ; ferme l'escrow |
| Après remboursement | plus aucun règlement, plus de dépôt |
| Frais de règlement | `SETTLEMENT_FEE` (10 000 lamports) par règlement, versés à celui qui soumet la transaction, prélevés sur le budget du fragment. Le requester les finance au dépôt : les nœuds ne paient rien |
| Arithmétique | toute opération est contrôlée (pas de débordement) |
| Gate de réputation | chaque approbateur a un compte de réputation on-chain (`VERIFIER_SEED`, `RegisterVerifier`) dont le score est ≥ `VERIFIER_ELIGIBLE` (2 500) ; sinon `IneligibleVerifier` (ADR 0006) |
| Crédit | un règlement accepté ajoute `VERIFIED_STEP` (100) au score de chaque approbateur, plafonné à 10 000 |

Invariants vérifiés par tirage aléatoire (40 graines × 300 opérations) : `payé + remboursé ≤
déposé`, `payé = budget des fragments réglés`, solde nul après fermeture.

## Réputation des vérificateurs on-chain (ADR 0006)

- Compte : PDA `[b"verifier", clé]`, 8 octets (`i64` LE). Créé par `RegisterVerifier` (payé par
  le signataire) à `VERIFIER_START` = 5 000. Une clé n'a qu'un compte.
- `settle` : compte de réputation passé pour chaque approbateur, après les approbateurs. Sans
  compte, ou sous le seuil : le règlement est refusé, rien n'est payé.
- Échelle reprise de V0 (départ 5 000, seuil 2 500, bornes 0 à 10 000). Pas de moyenne
  bayésienne : deux constantes et une règle lisibles.
- Il n'y a plus de comité (ADR 0007) : les approbateurs signent leur propre règlement. Une pénalité reste possible, signée par le requester (ADR 0008).
  décision séparée.
- Testé : règles pures dans `sil-chain-core` (`reputation`, `penalty`, `tests_reputation`) ;
  programme et adaptateur compilés et vérifiés hors localnet, pas de test localnet (pas de
  `solana-test-validator` sur le poste de développement).

## Message signé par les approbateurs

Disposition fixe (reproductible sur la chaîne sans décodeur CBOR) :

```
job(32) ‖ fragment u32 LE ‖ tentative u32 LE ‖ résultat(32) ‖ claimant(32) ‖ n u8 ‖ n × approbateur(32)
```

précédée du préfixe de domaine `"sil/v1/settlement" ‖ 0x00`. Les approbateurs sont les V premiers
par ordre de clé (tous les nœuds calculent la même liste, donc le même `IntentId`).

## Outbox (`sil-settlement`)

| Étape | Signification pour l'utilisateur |
|---|---|
| `Pending` | à soumettre, ou en attente des signatures des approbateurs |
| `Submitted` | acceptée par le réseau, **pas confirmée** |
| `Unknown` | la chaîne ne répond plus depuis 2 min : on ne sait pas |
| `Blocked` | le payeur n'a pas de SOL pour les frais, ou l'escrow est introuvable |
| `Confirmed` | **lu sur la chaîne** : le fragment est réglé comme voulu |
| `Rejected` | définitif : réglé autrement, ou le programme refuse |

Garanties, chacune avec un test contre une chaîne qui tombe en panne :

- **Idempotente** : un règlement a un identifiant unique ; le remettre en file fusionne les
  signatures sans doublon.
- **Relecture avant tout envoi** : l'escrow est lu avant `submit`. Une transaction qui a atterri
  malgré une erreur réseau (réponse perdue) n'est pas renvoyée.
- **Transaction perdue** : `Expired` → retour à `Pending` → relecture → renvoi (payé une fois).
- **429 / délai** : attente exponentielle (1 s → 60 s), puis `Unknown`, puis reprise.
- **Plusieurs nœuds** : chaque nœud peut soumettre le même règlement ; le second
  échoue côté programme (`AlreadySettled { same: true }`) et est reconnu comme `Confirmed`.
- **Redémarrage** : sauvegarde atomique (temporaire, `fsync`, renommage) ; un fichier corrompu est
  une **erreur**, jamais une outbox vide (V0 remplaçait en silence).
- **Mémoire** : au plus 4096 règlements suivis.

## Garde d'escrow (`check_escrow`)

Avant de travailler, un worker compare l'escrow au `JobSpec` signé : même job, même requester,
approbateurs, nombre de fragments, récompenses, échéance ; escrow ouvert et assez financé ; assez
de temps avant l'échéance. Sans cela un requester pourrait annoncer des approbateurs dans le spec et en
ouvrir un autre sur la chaîne.

## Programme on-chain : ce qui reste à écrire

**Rien de ce qui suit n'est compilé ni testé** : la chaîne d'outils Solana/Anchor n'est pas
installée sur le poste de développement. Le programme doit être une coquille mince autour de
`sil-chain-core` :

| Instruction | Signataires | Appelle |
|---|---|---|
| `open_escrow(job, terms, deposit)` | requester | `Escrow::open`, puis transfert du dépôt vers un compte PDA |
| `top_up(amount)` | requester | `Escrow::top_up` |
| `settle_fragment(args)` | tout payeur de frais | `Escrow::settle` avec les signataires lus dans les instructions **Ed25519 précompilées** de la même transaction (une par membre, message = disposition ci-dessus), puis transferts |
| `refund()` | requester | `Escrow::refund`, puis transfert du solde |

À respecter : PDA de l'escrow dérivé de `job_id` ; vérifier que chaque instruction précompilée porte
**le message exact** reconstruit depuis les arguments ; refuser les précompilés dont la clé n'est pas
**nouvel identifiant de programme** pour V1 ; autorité de mise à jour sous
**multisig** documentée ; audit externe avant tout fonds réels.

## Programme et adaptateur (écrits, testés en localnet)

Programme : `chain/program` (natif, `solana-program` 3). Instructions `open`, `top_up`, `settle`,
`refund` (format `EscrowIx` en bincode, dans `sil-chain-core::wire`). `settle` reconstruit le
message `settlement_frame` à partir de l'escrow et des arguments, puis ne compte que les signataires
dont un précompilé Ed25519 de la **même transaction** porte exactement ce message. Les payés doivent
être fournis en comptes inscriptibles ; leur part est déduite du solde de l'escrow.

Adaptateur : `chain/adapter` (`SolanaChain`, implémente `Chain`). Lecture du compte par PDA
`[escrow, job_id]`, construction de la transaction (un précompilé par signature, puis `settle`),
envoi avec preflight au niveau `confirmed`, suivi par `getSignatureStatuses`. Erreurs : frais
insuffisants : `NoFunds` ; erreur du programme : `Rejected(texte de la règle)` ; le reste : `Transient`.

Tests : `chain/adapter/tests/localnet.rs`, contre `solana-test-validator`, via
`chain/scripts/localnet.sh`. Résultat : ouverture, règlement payé (claimant : 1 000 lamports),
double règlement refusé, signature insuffisante refusée, signature d'un non-membre refusée,
dépôt complémentaire, remboursement avant échéance refusé, remboursement après échéance accepté,
job non rouvrable.

## Signatures et taille de transaction

- Chaque approbateur signe le règlement de son approbation une fois, et la signature est portée par son `Attest`.
  règlement n'est pas réglé (toutes les 8 s). Une signature perdue au passage ne bloque donc pas.
- L'adaptateur envoie **une** instruction Ed25519 pour les signatures (message stocké une fois,
  chaque entrée pointe dessus), et seulement `V` signatures valides d'approbateurs : la transaction reste
  sous 1232 octets, même avec des approbateurs nombreux (au plus 16).
- Test de bout en bout : `chain/e2e` (flux complet sur validateur local, montants contrôlés).

## Limites connues

- Programme écrit et testé sur `solana-test-validator` uniquement ; **pas audité, pas déployé**.
- Clé d'autorité de mise à jour : pas de multisig encore.
- Les signatures de règlement voyagent avec les `Attest` (journal). Elles ne sont pas des messages gossip séparés.
  faut un type de message gossip (ou un champ du `Finalize`) pour les collecter (phase 6).
- Les frais de règlement sont fixes (`SETTLEMENT_FEE`). Les montants suivent les unités du fragment, donc la taille du travail. Pas de bonus de qualité ni de malus de retard.
- Un nœud sans SOL ne soumet pas (`NoFunds`, outbox `Blocked`) : il ne paie jamais de frais de sa poche, mais il ne peut pas soumettre non plus si la clé payer est vide.
- Le remboursement fixe est de 24 h de grâce ; pas de contestation d'un règlement signé à tort par
des approbateurs complices : la chaîne fait confiance à `V` signatures d'approbateurs éligibles.
