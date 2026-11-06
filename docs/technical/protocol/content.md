# Contenu, modèles et transfert (`sil-content`, `sil-models`, `sil-transfer`)

## Identifiant

Le contenu est découpé en morceaux de **256 Kio**. Chaque morceau a un hash de feuille
(`SHA-256(0x00 ‖ morceau)`), les feuilles forment un arbre de Merkle
(`SHA-256(0x01 ‖ gauche ‖ droite)`, un nœud sans frère est remonté sans duplication).
L'identifiant est `SHA-256(0x03 ‖ taille ‖ racine)` : **il engage la taille**. Un contenu vide
a une racine dédiée. Les préfixes empêchent une feuille de passer pour un nœud.

Le **manifeste** (`taille`, liste des feuilles) se vérifie contre l'identifiant seul : on
peut l'obtenir d'un pair non fiable.

## Magasin (`ContentStore`)

| Opération | Coût |
|---|---|
| `has`, `size` | recherche en mémoire (V0 : relire et hasher jusqu'à 64 Mio) |
| `chunk(id, i)` | lit un seul morceau et le vérifie contre le manifeste |
| `put_reader` | flux, un morceau en mémoire à la fois |
| `begin_download` | vérifie manifeste et taille, réserve la place |

Disque : `objects/<2 hex>/<id>.data` + `.manifest` (le manifeste est écrit après les
données : sa présence signifie « complet »), `tmp/` pour les écritures en cours. L'index
mémoire se reconstruit au démarrage depuis les manifestes, sans relire les données.

**Quotas** : `max_total_bytes` (objets + téléchargements réservés) et `max_object_bytes`.
Quand la place manque, les objets **non épinglés** les moins récemment utilisés sont évincés ;
si tout est épinglé, `QuotaExceeded`. Épingler = `pin(id)` (contenu d'un job actif).

## Téléchargement reprenable (`Download`)

Chaque morceau est vérifié **avant** d'être écrit : un faux morceau est refusé, rien n'est
écrit. L'état (`.part` + `.have`) est sur disque : après un arrêt, `begin_download` reprend
avec les morceaux déjà valides. La place est réservée dès le début et rendue si le
téléchargement est abandonné.

## Transfert (`sil-transfer::fetch`)

Pour une liste de pairs : manifeste → vérification → morceaux par fenêtres parallèles
(4 par défaut). Un pair qui échoue, est saturé ou ment est abandonné ; le suivant reprend
**là où le précédent s'est arrêté**. Succès : le nœud s'annonce fournisseur. `StoreProvider`
expose un magasin au réseau.

`fetch` ne décide pas quoi télécharger : l'appelant vérifie la politique avant.

## Modèles (`sil-models`)

Catalogue TOML : un modèle = fichiers épinglés `(chemin, id, taille)`.

```toml
[[model]]
name = "e5-small"
[[model.file]]
path = "model/weights.bin"
id = "<64 hex>"
size = 123
```

Chemins relatifs sûrs (`[A-Za-z0-9._-]` et `/`, jamais `.` ni `..`), pas de doublons,
clés inconnues refusées. `Catalog::is_pinned(id)` est la question posée avant de télécharger
ou de servir un modèle ; `missing`/`is_ready` comparent au magasin.

## Téléchargements concurrents

Deux téléchargements du même contenu écrivaient le même fichier partiel (découvert en test de bout
en bout : deux fragments partageant une entrée). `begin_download` refuse désormais un second
téléchargement tant que le premier existe (`ContentError::InProgress`), et le nœud en sérialise un
par contenu pour que les suivants attendent au lieu d'échouer.

## Limites connues

- Pas de téléchargement depuis plusieurs pairs **en même temps** pour un même morceau
  (un pair à la fois, fenêtre de morceaux en parallèle).
- L'éviction compte les octets, pas l'espace réellement occupé par les fichiers partiels
  abandonnés dans `tmp/` (pas de nettoyage automatique).
- `Download::write_chunk` réécrit le fichier `.have` entier à chaque morceau (au plus
  quelques Kio pour 1 Gio).
- Le magasin n'est pas encore relié à la politique des jobs (épinglage automatique).
