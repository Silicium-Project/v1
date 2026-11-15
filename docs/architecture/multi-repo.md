# ADR — Dépôts indépendants (v1)

Le monorepo v1 est remplacé par un méta-dépôt public et des composants épinglés par SHA.

- Le méta-dépôt porte l'architecture, les ADR, `components.lock.json` et `.gitmodules`.
- Chaque composant a son dépôt, son historique et sa CI. Le réseau est public ; les autres composants sont privés.
- Une mise à jour de composant : PR dans le composant, puis PR dans le méta-dépôt qui met à jour le SHA
  (gitlink et `components.lock.json` ensemble).
- Aucun secret, clé de déploiement, certificat de signature ni `node.seed` ne se trouve dans un dépôt.
