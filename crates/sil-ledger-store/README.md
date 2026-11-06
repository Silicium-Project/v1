# sil-ledger-store

Segments append-only avec contrôle, réparation de fin interrompue, fsync configurable.

Dépend de : `sil-ledger`, `sil-crypto`. Corruption au milieu = erreur, jamais de réparation silencieuse.

Doc technique : ../../docs/technical/protocol/storage.md
