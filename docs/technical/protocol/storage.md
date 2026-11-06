# Stockage du journal (`sil-ledger-store`)

Dossier de segments `seg-00000001.log`, `seg-00000002.log`… (ajout seul). Un segment est
fermé quand il dépasse `max_segment_bytes` (8 Mio par défaut).

## Enregistrement

```
longueur u32 LE | événement canonique (sil-codec) | 4 octets = début de SHA-256 de l'événement
```

## Ouverture

`EventStore::open` rejoue tous les événements dans l'ordre d'écriture via un rappel (le
nœud les donne à `Ledger::receive`, qui revérifie signatures et quotas), puis ouvre le
segment courant en ajout.

| Situation | Comportement |
|---|---|
| Fin du **dernier** segment incomplète ou au contrôle faux | écriture interrompue : tronquée, `repaired_tail = true` |
| Enregistrement incomplet dans un segment plus ancien | `Corrupt`, aucune modification |
| Contrôle faux au milieu, longueur absurde | `Corrupt`, aucune modification |

Une corruption n'est jamais réparée en silence : l'opérateur décide (restaurer, ou
laisser les pairs rétablir les événements manquants par rattrapage).

## Durabilité (`FsyncPolicy`)

`Always` (rien n'est perdu), `EveryN(n)` (défaut 32), `Never` (tests). Un événement
perdu à cause d'un arrêt brutal est récupérable auprès des pairs.

## Limites connues

- Pas de snapshot ni de compaction : le démarrage rejoue (et revérifie) tout. À traiter
  avec les mesures de la phase 6.
- La persistance se fait **après** l'acceptation en mémoire (V0 : avant). Un arrêt entre
  les deux perd l'événement, qui revient par rattrapage.
- Les segments ne sont pas supprimés après la fin d'un job.
