# ADR 0006 : réputation des vérificateurs on-chain, gate au règlement

**Statut :** accepté (choix A du 8 novembre 2026). **Mis à jour par ADR 0007 :** les pénalités du comité sont supprimées ; ADR 0008 les rétablit, signées par le requester.

## Contexte

ADR 0005 retire le vote de coordinateur : les vérificateurs sont les premières `k` offres valides
d'auteurs distincts du claimant. La liste blanche `SIL_API_COMMITTEE` (correction du risque
« vérificateurs Sybil ») a réintroduit un comité fixe : seuls ses membres peuvent vérifier. Un job
peut alors bloquer, faute de membres en ligne (constaté le 8 novembre, job `3e0763…`).

V0 ne bornait pas la sélection par une liste. Tout nœud à rôle `verify` pouvait vérifier, et la
réputation intervenait à deux endroits :

- en P2P, le publisher accepte les offres ;
- on-chain, `vote_fragment` exige `verify_reputation ≥ MIN_VERIFY_REPUTATION` (2 500) sur le compte
  `Worker` du votant (`chain/solana/programs/solana-layer/src/instructions/vote_fragment.rs:50`).

En V1, la réputation dérivée du journal (`sil-reputation`) ne pèse pas sur la sélection. Le réducteur
doit être déterministe sur le journal ; l'état Solana n'y figure pas, donc il ne peut pas lire la
réputation on-chain sans que chaque nœud ait la même vue.

## Décision

Le contrôle de réputation se fait **au règlement**, sur la chaîne :

1. Un compte de réputation par vérificateur, PDA `[b"verifier", clé]`, score `i64` à 5 000.
   `RegisterVerifier` le crée, payé par le signataire.
2. `Settle` lit le score de chaque approbateur. Sans compte, ou sous 2 500, le règlement est refusé
   (`IneligibleVerifier`). Rien n'est payé.
3. Un règlement accepté ajoute 100 au score de chaque approbateur (plafond 10 000).
4. `Penalize`, signé par le comité (`threshold` signatures, message `penalty_frame`), retire 1 000 à
   un approbateur d'un fragment réglé. Une fois par (fragment, vérificateur). Trois fautes sortent un
   vérificateur de l'éligibilité.

La sélection P2P reste ouverte : premières `k` offres valides, sans liste blanche. Un vérificateur
non éligible peut signer des offres, mais le règlement ne le paiera pas.

## Règles de l'ADR 0005 modifiées

- Règle 4 (vérificateurs) : pas de liste blanche. `SIL_API_COMMITTEE` ne filtre plus les offres
  de vérification, ni dans le réducteur (`attempt.rs`, `finalize.rs`, `open.rs`) ni dans le nœud
  (`verifier_loop.rs`). Il reste la liste des signataires de règlement et des sanctions.
- Le réducteur et le journal ne changent pas : aucune donnée Solana n'entre dans l'état P2P.

## Conséquences

- Le comité reste l'autorité de règlement et de pénalité. Retirer le comité du règlement est une
  décision séparée : il faudra alors une autre preuve de règlement que les signatures du comité.
- Coût d'entrée : rent d'un compte (≈ 0,001 SOL) et des frais de transaction, payés une fois.
- Une pénalité suppose que le comité ait observé la faute. Ce critère n'est pas encore automatisé.
- Le nœud n'enregistre pas encore ses clés : `register_verifier_ix` existe dans l'adaptateur, mais
  rien ne l'appelle. Sans enregistrement, les règlements d'un vérificateur sont refusés.
- Un job déjà ouvert garde son comité dans sa spec : ce changement ne débloque pas un job bloqué.

## Alternatives écartées

- **B, snapshot dans le journal** : l'offre porte la réputation lue on-chain, et le réducteur filtre
  dessus. Pas de comité dans la sélection, mais le snapshot est déclaratif : un nœud peut le
  falsifier sans preuve de compte.
- **Réputation dérivée du journal seule** : déjà en place (phase 1b), non utilisée pour la sélection.
  Ce n'est pas une réputation publique ni vérifiable par le payeur.

## Vérification

- `sil-chain-core` : règles pures testées (`reputation`, `penalty`, `tests_reputation`,
  `tests_random`).
- `chain/program` et `chain/adapter` : compilés et testés hors localnet. `cargo build-sbf` et
  `solana-test-validator` ne sont pas installés sur le poste de développement.
