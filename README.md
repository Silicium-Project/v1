# Silicium v1

Méta-dépôt : architecture, décisions (ADR), contrats partagés et composition des composants.

Le code est réparti en dépôts indépendants, épinglés par SHA dans `components.lock.json` et
`.gitmodules` :

| Composant | Dépôt | Visibilité |
|---|---|---|
| Réseau (protocole, ledger, nœud) | `v1-network` | public |
| Application de bureau | `v1-desktop` | privé |
| Site | `v1-web` | privé |
| API (comptes, passerelle) | `v1-api` | privé |
| Programme Solana, scripts de déploiement | `v1-chain` | privé |

Décisions d'architecture : [`docs/architecture/`](docs/architecture/).

Branches : `main` = production, `dev` = intégration et site de développement.
