# sil-raytracer

Moteur de rendu raytracing du workload `rendu/raytracer`, porté de `silicium-raytracer` (v0,
moteur 2). Une scène `.cfg` devient des pixels RGB24, déterministes : deux machines qui rendent
la même bande produisent les mêmes octets.

## Ce qui est porté

| Module | Contenu |
|---|---|
| `syntax` | lecture de la syntaxe des scènes (blocs, listes, commentaires, keyframes) |
| `scene` | construction de la scène pour une image : réglages, caméra, matières, primitives, lumières |
| `geometry` | sphères, plans, triangles, quads, transformations, BVH |
| `integrator` | lancer de rayons : Whitted et path tracing, tone mapping |
| `random` | aléatoire déterministe (SplitMix64) et fonctions de `libm` |
| `render` | `render_rows` : une bande de lignes, mono-thread |
| `bin/sil-raytracer-block` | protocole de `ExternalRenderer` : args `<l> <h> <y0> <y1>`, scène sur l'entrée, pixels sur la sortie |

Les tests reprennent les sorties figées de la v0 (empreintes SHA-256 de deux scènes) : le portage
produit les mêmes octets.

## Hors de ce crate, pour l'instant

- la vérification de scène (`check`), à porter avec l'éditeur ;
- l'import `.obj` (les scènes du site contiennent déjà leurs `mesh`) ;
- le pipeline de tuiles et jalons de la v0, remplacé par les bandes et la vérification par
  échantillon de `sil-wl-render`.

## Règles de déterminisme

- Pas de thread dans le rendu de bande : chaque pixel ne dépend que de ses coordonnées et de la scène.
- Fonctions transcendantes via `libm` (Rust pur), pas via la libc de la machine.
- Toute modification des pixels change les empreintes des tests : c'est voulu. Une telle
  modification exige une nouvelle version du moteur (`ENGINE_VERSION`).

## Utilisation

```bash
cargo build --release -p sil-raytracer
printf 'camera : { resolution = { width = 16 height = 10 } }' | \
  ./target/release/sil-raytracer-block 16 10 0 10 > bande.rgb
```

La scène déclare sa résolution (`camera > resolution`) : elle doit être égale à `<l> <h>`.
