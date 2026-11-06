# sil-quota

Plafonds du journal (`LedgerLimits`) et limiteurs de débit à mémoire bornée (`TokenBucket`, `KeyedLimiter`).

Dépend de : rien. Invariant : une nouvelle clé ne peut pas évincer le seau épuisé d'une autre.

Doc technique : ../../docs/technical/protocol/ledger.md
