# ADR 0014 — Migration du journal entre versions

**Statut** : proposée. Aucun code de migration n'existe encore. Décision à valider avant implémentation.

## Contexte

Le journal (`ledger/seg-*.log`) stocke les événements signés tels quels. Un changement de format
rend l'ancien journal illisible par la nouvelle version : `encodage non canonique` au démarrage.
C'est arrivé avec la 1.8 (claims ancrés, verdicts en trame binaire fixe). Les nœuds ont dû être
remis à zéro, et leur historique local (gains en attente, réputation locale) a été perdu.

Le numéro `EVENT_VERSION` (u8, `crates/sil-ledger/src/event.rs`) existe, mais il ne dit pas comment
lire les anciennes versions. Aucun décodeur ancien n'est conservé.

## Contraintes

1. **Les événements sont signés.** Une migration ne peut pas re-signer un événement : elle doit
   conserver les octets d'origine, ou réécrire avec une autorité explicite. Un événement réécrit
   sans sa signature n'est plus une preuve.
2. **Les règles changent avec les versions.** Un claim ancré (1.8) n'a pas de sens sous les règles
   de 1.7 : la fraîcheur se juge à la publication, pas à la relecture.
3. **Aucune migration automatique au démarrage** (règle de `docs/technical/backup.md`, point 3).
4. **Sauvegarde vérifiée avant toute migration** (protocole existant, `infra/vps/backup.sh`).

## Options

**A. Décodeurs versionnés + outil de migration hors ligne.** Le journal porte un en-tête de version.
Le nœud garde les décodeurs des versions encore présentes. `silicium-migrate` lit un ancien journal,
vérifie chaque signature avec son décodeur, et écrit un journal neuf (même contenu, format neuf).
Les événements dont les règles ont changé sont recopiés comme **historique clos** (marqués, jamais
rejoués comme claims actifs).

**B. Instantané signé.** Le nœud signe un état (réputation, fragments finalisés, règlements en
attente) à un instant donné et ne garde que l'instantané et les événements après lui. Plus simple à
faire évoluer, mais la preuve historique est perdue.

**C. Reset (situation actuelle).** Acceptable pour un réseau de test, pas pour un réseau réel.

## Décision proposée

Option **A** pour la transition, puis **B** comme format de fond une fois la réputation et les
règlements stabilisés.

Règles :
- l'en-tête du journal porte `format_version` ; un nœud refuse un journal dont la version lui est
  inconnue, avec un message qui renvoie vers `silicium-migrate` ;
- `silicium-migrate` n'écrit jamais sur l'original : il crée un dossier `ledger.v<N>` à côté et
  affiche les sommes avant et après ;
- chaque migration est précédée d'une sauvegarde vérifiée ;
- les décodeurs des anciennes versions sont gelés (tests de non-régression sur des fichiers réels,
  sous `crates/sil-ledger/tests/fixtures/`).

## Conséquences

- Les nœuds gardent leur identité (`node.seed`) et leur historique signé pendant une mise à jour.
- Coût : maintenir des décodeurs gelés, et des fixtures par version.
- Le nombre de versions à migrer doit rester borné (une précédente, pas plus).
