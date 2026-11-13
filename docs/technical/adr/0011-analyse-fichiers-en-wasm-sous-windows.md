# ADR 0011 — Analyse des fichiers en WASM sous Windows

**Statut** : décidée, non implémentée.

## Contexte

Sous Linux, le rendu tourne dans une sandbox forte (landlock, limites de ressources,
`sil-sandbox-launch`). Sous Windows, l'isolation est « best-effort » : seul l'arrêt de l'arbre de
processus est garanti. Le risque restant est un bug dans l'analyse d'un fichier non fiable (scène
`.cfg`, image).

## Décision

Sous Windows, l'analyse des fichiers non fiables s'exécute dans un module WASM, pas dans le processus
natif. Le module n'a aucun accès au système de fichiers, au réseau ni à l'horloge, et reçoit des
limites de mémoire et de temps. Un bug d'analyse reste confiné au module.

- La crate `sil-render-wasm` porte ce module. Le test `tests/raytracer_parity.rs` vérifie que le
  résultat WASM est identique au rendu natif.
- Le nœud Windows appelle le module pour l'analyse, puis passe les données validées au rendu.
- Linux garde la sandbox existante.

## Conséquences

- La limite actuelle (« un rendu non vérifié n'est pas isolé sous Windows ») disparaît une fois le
  module en place pour tous les workloads concernés.
- Coût : le rendu WASM est plus lent que le natif. La parité de résultat est une condition, pas la
  vitesse.
- Le rendu lui-même peut rester natif si seule l'analyse est isolée ; à trancher selon les mesures.

## Points ouverts

- Liste des workloads dont l'analyse passe en WASM (`rendu/raytracer`, `image/batch`).
- Moteur WASM retenu et limites de mémoire et de temps.
