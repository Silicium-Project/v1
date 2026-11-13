# Garanties et modèle de confiance

Ce document dit ce que le réseau garantit, ce qu'il ne garantit pas, et qui il faut croire. Il sert de
référence aux réponses sur la sécurité et le paiement.

## 1. Exactly-once : ce qui est vrai

Le réseau ne garantit pas l'exactly-once du calcul. Il garantit « au moins une fois » pour l'exécution
et « une fois » pour le paiement.

- **Exécution** : un fragment réclamé dont le bail expire est remis en jeu. Il peut donc être calculé
  deux fois, par deux nœuds. Un seul résultat est retenu par le journal.
- **Événements** : chaque événement est identifié par son hash. Un doublon reçu est ignoré.
- **Paiement** : le programme Solana règle un fragment une seule fois. Un second règlement du même
  fragment est refusé par l'état de l'escrow. Le calcul peut donc être refait ; le paiement, non.

## 2. Qui fait foi

- **Argent** : la chaîne Solana fait foi. Un fragment n'est payé qu'après un règlement signé par ses
  vérificateurs, et seulement après finalisation (`finalized`). Un gain affiché avant cela n'est pas
  acquis.
- **Journal des tâches** : chaque nœud le reconstruit à partir des événements signés. Pendant une
  partition réseau, deux côtés peuvent avoir des vues différentes. Le règlement on-chain tranche ;
  le journal se réconcilie ensuite sur l'état on-chain.
- **Ordre des claims** : aujourd'hui, l'ordre utilise l'heure déclarée par l'auteur. Un worker peut
  donc la décaler. L'ancrage sur un slot Solana est en place (ADR 0009) : l'ordre suit le slot signé, borné à la réception.

## 3. Modèle de confiance : le VPS est un opérateur central

Le réseau n'est pas aussi décentralisé que l'architecture le suggère. Aujourd'hui, le VPS :

- fait tourner l'API (comptes, soumission des jobs, site) ;
- sert de relais de règlement pour les nœuds sans clé de paiement ;
- est un nœud d'amorçage et de relais P2P.

Ce qu'il peut faire : refuser des requêtes, retarder des événements, ou bloquer l'accès au site.
Ce qu'il ne peut pas faire : détourner les fonds de l'escrow ni payer un worker qui n'a pas réglé
son fragment. Ces règles sont dans le programme Solana, publiques et vérifiables.

Un audit indépendant de la chaîne reste possible sans faire confiance au VPS : les soldes et
règlements se vérifient sur l'explorateur Solana.

## 4. Isolation du calcul

Les utilisateurs ne soumettent pas de code : les workloads sont une liste fixe (`calcul/sha256`,
`rendu/raytracer`, `embedding/e5-small`, `image/batch`). Le risque restant est un bug dans l'analyse
d'un fichier (scène `.cfg`, image).

- **Linux** : isolation forte (landlock, limites de ressources, lanceur `sil-sandbox-launch`).
- **Windows** : isolation « best-effort ». Seul l'arrêt de l'arbre de processus est garanti. Un rendu
  non vérifié n'est donc pas isolé sous Windows.

Les nœuds Windows doivent être considérés comme de confiance pour les calculs non vérifiés, tant que
cette limite n'est pas corrigée.

## 5. Ce qui reste ouvert

- Un même propriétaire peut contrôler plusieurs clés et voter pour lui-même. La caution par vérificateur
  est décidée (ADR 0010 : 50 % de la récompense retenus, libérés après 20 règlements ou 24 h),
  pas encore en place.
- L'ordre des claims suit le slot Solana signé (ADR 0009) ; la borne de réception dépend de la lecture RPC locale.
- L'isolation Windows est faible (voir section 4). Correctif décidé : analyse des fichiers en WASM
  (ADR 0011), pas encore en place.
