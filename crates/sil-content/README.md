# sil-content

Contenu adressé par Merkle (morceaux de 256 Kio), index mémoire, quotas avec éviction LRU, téléchargement reprenable.

Dépend de : `sil-types`, `sil-crypto`, `sil-codec`. `has` ne lit pas le disque ; chaque morceau est vérifié avant écriture.

Doc technique : ../../docs/technical/protocol/content.md
