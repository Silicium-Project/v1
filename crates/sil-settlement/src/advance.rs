//! Un pas de l'automate d'un règlement.
//!
//! Règle d'or : **on relit l'état de l'escrow avant tout envoi**. Une transaction qui a
//! atterri alors que l'appelant a reçu une erreur (délai, 429) est donc reconnue comme
//! réglée au lieu d'être renvoyée.

use crate::{
    Chain, ChainError, OutboxConfig, TxStatus,
    outbox::{Entry, Phase},
};
use sil_chain_core::Escrow;

fn contact_ok(entry: &mut Entry) {
    entry.failures = 0;
    entry.first_failure_ms = None;
}

fn transient(entry: &mut Entry, config: &OutboxConfig, now_ms: u64) {
    entry.failures = entry.failures.saturating_add(1);
    entry.first_failure_ms.get_or_insert(now_ms);
    entry.next_try_ms = now_ms.saturating_add(config.backoff(entry.failures));
}

/// Verdict de la lecture de l'escrow pour ce règlement.
enum Settled {
    NotYet,
    AsWanted,
    Otherwise,
}

fn settled(entry: &Entry, escrow: &Escrow) -> Settled {
    match escrow.settled(entry.auth.claim.fragment) {
        None => Settled::NotYet,
        Some(done) if entry.auth.claim.matches(done) => Settled::AsWanted,
        Some(_) => Settled::Otherwise,
    }
}

/// Avance d'un pas. Retourne vrai si l'étape a changé.
pub(crate) fn step(
    entry: &mut Entry,
    chain: &dyn Chain,
    config: &OutboxConfig,
    now_ms: u64,
) -> bool {
    if entry.phase.is_terminal() || now_ms < entry.next_try_ms {
        return false;
    }
    let before = entry.phase.clone();
    match before.clone() {
        Phase::Submitted { tx, .. } => on_submitted(entry, chain, config, now_ms, &tx),
        _ => on_pending(entry, chain, config, now_ms),
    }
    entry.phase != before
}

fn on_pending(entry: &mut Entry, chain: &dyn Chain, config: &OutboxConfig, now_ms: u64) {
    let job = entry.auth.claim.job;
    let escrow = match chain.escrow(&job) {
        Ok(Some(escrow)) => escrow,
        Ok(None) => {
            contact_ok(entry);
            entry.phase = Phase::Blocked {
                reason: "escrow introuvable sur la chaîne".into(),
            };
            entry.next_try_ms = now_ms.saturating_add(config.poll_ms * 5);
            return;
        }
        Err(error) => {
            tracing::debug!(%error, "règlement : lecture de l'escrow échouée");
            return transient(entry, config, now_ms);
        }
    };
    // Lire l'escrow ne prouve pas que la soumission suivante passera : le compteur d'échecs ne
    // repart à zéro qu'avec un vrai progrès, sinon le délai ne grandit jamais.
    match settled(entry, &escrow) {
        Settled::AsWanted => {
            contact_ok(entry);
            entry.phase = Phase::Confirmed;
            return;
        }
        Settled::Otherwise => {
            entry.phase = Phase::Rejected {
                reason: "fragment déjà réglé avec d'autres paramètres".into(),
            };
            return;
        }
        Settled::NotYet => {}
    }
    let signers = entry.auth.approver_signers();
    let needed = entry.auth.claim.approvers.len();
    if signers.len() < needed {
        // En attente de signatures des approbateurs : rien à soumettre encore.
        contact_ok(entry);
        entry.phase = Phase::Pending;
        entry.next_try_ms = now_ms.saturating_add(config.poll_ms);
        return;
    }
    // Seules les signatures des approbateurs partent (une par approbateur).
    let mut minimal = entry.auth.clone();
    minimal
        .signatures
        .retain(|(signer, _)| signers.iter().take(needed).any(|chosen| chosen == signer));
    match chain.submit(&minimal) {
        Ok(tx) => {
            contact_ok(entry);
            entry.attempts += 1;
            entry.phase = Phase::Submitted { tx, at_ms: now_ms };
            entry.next_try_ms = now_ms.saturating_add(config.poll_ms);
        }
        // Peut avoir atterri malgré l'erreur : le prochain pas relit l'escrow avant de renvoyer.
        Err(ChainError::Transient(reason)) => {
            tracing::debug!(%reason, "règlement : soumission échouée, nouvel essai");
            transient(entry, config, now_ms)
        }
        Err(ChainError::NoFunds) => {
            entry.phase = Phase::Blocked {
                reason: "pas de fonds pour les frais de transaction".into(),
            };
            transient(entry, config, now_ms);
        }
        Err(ChainError::Rejected(reason)) => on_refusal(entry, chain, reason),
    }
}

fn on_submitted(
    entry: &mut Entry,
    chain: &dyn Chain,
    config: &OutboxConfig,
    now_ms: u64,
    tx: &crate::TxId,
) {
    match chain.tx_status(tx) {
        Ok(TxStatus::Confirmed) => {
            contact_ok(entry);
            entry.phase = Phase::Confirmed;
        }
        Ok(TxStatus::Pending) => {
            contact_ok(entry);
            entry.next_try_ms = now_ms.saturating_add(config.poll_ms);
        }
        // Ne peut plus atterrir : retour à l'étape « à soumettre » qui relit l'escrow d'abord.
        Ok(TxStatus::Expired) => {
            contact_ok(entry);
            entry.phase = Phase::Pending;
            entry.next_try_ms = now_ms;
        }
        Ok(TxStatus::Failed(reason)) => {
            tracing::debug!(%reason, attempts = entry.attempts, "règlement : transaction échouée");
            contact_ok(entry);
            if entry.attempts >= config.max_attempts {
                entry.phase = Phase::Rejected { reason };
            } else {
                // Peut être passagère : on relira l'escrow puis on resoumettra.
                entry.phase = Phase::Pending;
                entry.next_try_ms = now_ms.saturating_add(config.backoff(entry.attempts));
            }
        }
        Err(_) => transient(entry, config, now_ms),
    }
}

/// Le programme a refusé : peut-être parce que c'est déjà réglé comme voulu.
fn on_refusal(entry: &mut Entry, chain: &dyn Chain, reason: String) {
    let job = entry.auth.claim.job;
    entry.phase = match chain.escrow(&job) {
        Ok(Some(escrow)) if matches!(settled(entry, &escrow), Settled::AsWanted) => {
            Phase::Confirmed
        }
        _ => Phase::Rejected { reason },
    };
}
