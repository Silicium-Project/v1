use sil_ledger::OpenOffer;
use sil_reputation::Reputation;

/// Trie les offres : meilleure réputation d'abord, puis la plus ancienne, puis le plus petit
/// identifiant. Ordre total : deux membres qui voient les mêmes offres choisissent pareil.
pub(crate) fn rank(offers: &mut [OpenOffer], reputation: &Reputation, now_ms: u64) {
    offers.sort_by(|a, b| {
        let (score_a, score_b) = (
            reputation.score(&a.author, now_ms),
            reputation.score(&b.author, now_ms),
        );
        score_b
            .total_cmp(&score_a)
            .then(a.issued_at_ms.cmp(&b.issued_at_ms))
            .then(a.event.cmp(&b.event))
    });
}
