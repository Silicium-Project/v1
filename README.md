# Silicium

Calcul distribué vérifiable. Des machines volontaires exécutent des tâches (calcul, embedding,
rendu), d'autres les vérifient, et le règlement se fait on-chain sur Solana. Aucun résultat n'est
accepté sans vérification, et aucune machine n'est payée sans avoir travaillé.

## Fonctionnement

1. Un requester publie un job et dépose l'escrow sur Solana.
2. Le job est découpé en fragments. Un worker réclame un fragment, en l'ancrant à un slot Solana
   (preuve d'horloge, [ADR 0009](docs/architecture/0009-preuve-d-horloge.md)).
3. Le worker calcule et publie son résultat.
4. Des vérificateurs recalculent un échantillon ; le quorum tranche
   ([ADR 0005](docs/architecture/0005-verifier-quorum.md),
   [ADR 0012](docs/architecture/0012-quorum-et-partage-de-la-caution.md)).
5. Le règlement paie les workers et les vérificateurs. Un vérificateur minoritaire perd une part de
   sa retenue ([ADR 0010](docs/architecture/0010-retenue-sur-recompense-verificateur.md)).
6. Une réputation bayésienne multidimensionnelle module l'attribution des tâches
   ([ADR 0013](docs/architecture/0013-reputation-multidimensionnelle-bayesienne.md)).

## Composants

Le code est réparti en dépôts indépendants, épinglés par SHA dans
[`components.lock.json`](components.lock.json).

| Composant | Rôle | Dépôt | Visibilité |
|---|---|---|---|
| Réseau | Protocole P2P, ledger signé, nœud, workloads (Rust) | `v1-network` | privé pour l'instant |
| Bureau | Application Tauri : installe et supervise le nœud (Windows, Linux) | `v1-desktop` | privé |
| Site | Site public et téléchargements | `v1-web` | privé |
| API | Comptes et passerelle vers le réseau (Go) | `v1-api` | privé |
| Chaîne | Programme Solana, scripts de déploiement | `v1-chain` | privé |

Le méta-dépôt (ce dépôt) porte l'architecture, les décisions et la composition.

## Architecture

Les décisions sont dans [`docs/architecture/`](docs/architecture/) :

- [0001](docs/architecture/0001-coordination-committee.md) à
  [0008](docs/architecture/0008-penalite-par-le-requester.md) : comité, certificat de finalisation,
  réputation par portée, vérification, pénalités.
- [0009](docs/architecture/0009-preuve-d-horloge.md) à
  [0013](docs/architecture/0013-reputation-multidimensionnelle-bayesienne.md) : preuve d'horloge,
  retenue, quorum, réputation.
- [0014](docs/architecture/0014-migration-du-journal.md) : migration du journal entre versions
  (brouillon).

## Statut

Réseau de test : le règlement tourne sur le devnet Solana, avec des jetons de test. Dernière version
publiée : 1.9.0 (nœud, bureau Windows et Linux).

## Développement

- Branches : `main` = production, versionnée par tags ; `dev` = intégration et site de développement ;
  les branches de travail partent de `dev` et reviennent par pull request.
- Chaque dépôt a sa CI. Règle commune : au plus 300 lignes par fichier, aucun secret versionné.
- Les commandes de build et de test sont dans le README de chaque dépôt (par exemple
  `cargo test --workspace` pour le réseau).

## Sécurité

Ne publiez pas de vulnérabilité en issue publique. Contactez les mainteneurs en privé.
Ne versionnez jamais de clé, de seed (`node.seed`), de token, de certificat de signature ou de
fichier `.env`.

## Licence

MIT (indiqué dans les manifestes du réseau).
