use crate::{mine::Mine, ranking::rank};
use sil_attest::{Certificate, MAX_APPROVALS};
use sil_crypto::{SigningIdentity, sha256};
use sil_ledger::{EventBody, Ledger, TaskState, TaskStatus};
use sil_reputation::Reputation;
use sil_types::{Hash32, JobId, PublicKey, TaskRef};

pub struct Coordinator {
    identity: SigningIdentity,
    claim_window_ms: u64,
    leader_wait_ms: u64,
}

/// Chef de tour d'un emplacement `(job, fragment, tentative)` : indice dans le comité, tiré du
/// hash de l'emplacement. Tous les membres calculent le même.
pub fn leader_index(job: &JobId, fragment: u32, attempt: u32, members: usize) -> usize {
    let mut data = Vec::with_capacity(40);
    data.extend_from_slice(job.as_bytes());
    data.extend_from_slice(&fragment.to_le_bytes());
    data.extend_from_slice(&attempt.to_le_bytes());
    let digest = sha256(&data);
    let mut head = [0u8; 8];
    head.copy_from_slice(&digest.as_bytes()[..8]);
    (u64::from_le_bytes(head) % members.max(1) as u64) as usize
}

impl Coordinator {
    /// Sans fenêtre d'observation : vote dès qu'un claim existe (tests, un seul membre).
    pub fn new(identity: SigningIdentity) -> Self {
        Self::with_window(identity, 0)
    }

    /// `claim_window_ms` : un membre attend que le plus ancien claim ouvert ait cet âge avant
    /// de voter, pour que tous les membres aient vu les mêmes offres. Les votes étant
    /// irrévocables, deux membres qui votent chacun pour « son » claim bloquent le fragment
    /// quand les membres vivants n'atteignent le seuil que tous ensemble.
    pub fn with_window(identity: SigningIdentity, claim_window_ms: u64) -> Self {
        Self {
            identity,
            claim_window_ms,
            leader_wait_ms: 0,
        }
    }

    /// Les membres autres que le chef de tour attendent son vote jusqu'à `ms` après la fin de la
    /// fenêtre d'observation, puis votent d'eux-mêmes (chef muet ou en panne). Évite que deux
    /// membres votent chacun pour « leur » claim au même instant.
    pub fn with_leader_wait(mut self, ms: u64) -> Self {
        self.leader_wait_ms = ms;
        self
    }

    pub fn public(&self) -> PublicKey {
        self.identity.public()
    }

    /// Identité de signature, pour que le nœud signe les votes retournés.
    pub fn identity(&self) -> &SigningIdentity {
        &self.identity
    }

    /// Votes que ce membre doit émettre maintenant pour `job`. Vide si ce n'est pas un
    /// membre du comité, si le job est inconnu, ou s'il n'y a rien à décider.
    pub fn decide(
        &self,
        ledger: &Ledger,
        reputation: &Reputation,
        job: &JobId,
        now_ms: u64,
    ) -> Vec<EventBody> {
        let Some(signed) = ledger.spec(job) else {
            return Vec::new();
        };
        let spec = &signed.spec;
        if !spec.committee.contains(&self.public()) {
            return Vec::new();
        }
        let mine = Mine(ledger.events_of(job, &self.public()));
        let mut votes = Vec::new();
        for fragment in 0..spec.fragments {
            let Some(state) = ledger.task_state(job, fragment, now_ms) else {
                continue;
            };
            if state.status.is_terminal() {
                continue;
            }
            let context = Context {
                ledger,
                reputation,
                job,
                fragment,
                now_ms,
                mine: &mine,
                required: spec.verifications_required,
                claim_window_ms: self.claim_window_ms,
                leader_wait_ms: self.leader_wait_ms,
                me: self.public(),
                committee: &spec.committee,
            };
            votes.extend(context.accept_claim(&state));
            votes.extend(context.accept_verifiers(&state));
            votes.extend(context.finalize(&state));
        }
        votes
    }
}

/// Ce qu'il faut savoir pour décider d'un fragment.
struct Context<'a> {
    ledger: &'a Ledger,
    reputation: &'a Reputation,
    job: &'a JobId,
    fragment: u32,
    now_ms: u64,
    mine: &'a Mine,
    required: u32,
    claim_window_ms: u64,
    leader_wait_ms: u64,
    me: PublicKey,
    committee: &'a [PublicKey],
}

impl Context<'_> {
    fn accept_claim(&self, state: &TaskState) -> Option<EventBody> {
        let attempt = state.next_attempt?;
        if self.mine.voted_claim(self.fragment, attempt) {
            return None;
        }
        let mut offers = self
            .ledger
            .open_claims(self.job, self.fragment, self.now_ms);
        let oldest = offers.iter().map(|o| o.issued_at_ms).min()?;
        if self.now_ms < oldest.saturating_add(self.claim_window_ms) {
            return None; // fenêtre d'observation : laisser arriver les autres claims
        }
        if self.must_wait_for_leader(attempt, oldest, &offers) {
            return None;
        }
        rank(&mut offers, self.reputation, self.now_ms);
        let claim = self
            .followed_claim(attempt, &offers)
            .unwrap_or(offers.first()?.event);
        Some(EventBody::Accept {
            fragment: self.fragment,
            attempt,
            claim,
        })
    }

    /// Un membre qui n'est pas le chef de tour attend son vote (et que son claim nous soit
    /// parvenu), tant que le délai d'attente n'est pas écoulé.
    fn must_wait_for_leader(
        &self,
        attempt: u32,
        oldest_ms: u64,
        offers: &[sil_ledger::OpenOffer],
    ) -> bool {
        if self.leader_wait_ms == 0 || self.committee.is_empty() {
            return false;
        }
        let index = leader_index(self.job, self.fragment, attempt, self.committee.len());
        let leader = self.committee[index];
        let deadline = oldest_ms
            .saturating_add(self.claim_window_ms)
            .saturating_add(self.leader_wait_ms);
        if leader == self.me || self.now_ms >= deadline {
            return false;
        }
        let votes = self.ledger.claim_votes(self.job, self.fragment, attempt);
        match votes.iter().find(|(who, _)| *who == leader) {
            None => true,
            Some((_, claim)) => !offers.iter().any(|offer| offer.event == *claim),
        }
    }

    /// S'aligne sur le choix déjà exprimé par d'autres membres : le claim le plus voté parmi
    /// ceux qu'on connaît (à égalité, le mieux classé). Ignore les votes pour un claim inconnu.
    fn followed_claim(&self, attempt: u32, offers: &[sil_ledger::OpenOffer]) -> Option<Hash32> {
        let votes = self.ledger.claim_votes(self.job, self.fragment, attempt);
        offers
            .iter()
            .map(|offer| {
                let count = votes
                    .iter()
                    .filter(|(_, claim)| *claim == offer.event)
                    .count();
                (count, offer.event)
            })
            .filter(|(count, _)| *count > 0)
            .fold(
                None,
                |best: Option<(usize, Hash32)>, candidate| match best {
                    Some((count, _)) if count >= candidate.0 => best,
                    _ => Some(candidate),
                },
            )
            .map(|(_, claim)| claim)
    }

    fn accept_verifiers(&self, state: &TaskState) -> Vec<EventBody> {
        let Some(last) = state.attempts.last() else {
            return Vec::new();
        };
        let slots = self
            .ledger
            .verification_slots_open(self.job, self.fragment, self.now_ms);
        if slots == 0 {
            return Vec::new();
        }
        // Mes votes en attente (offre pas encore retenue) occupent déjà des places.
        let accepted: Vec<_> = last.verifiers.iter().map(|v| v.offer_event).collect();
        let voted = self.mine.verifier_votes(self.fragment, last.attempt);
        let pending = voted.iter().filter(|o| !accepted.contains(o)).count();
        let to_fill = (slots as usize).saturating_sub(pending);
        let mut offers = self
            .ledger
            .open_verifications(self.job, self.fragment, self.now_ms);
        offers.retain(|offer| !voted.contains(&offer.event));
        rank(&mut offers, self.reputation, self.now_ms);
        offers
            .into_iter()
            .take(to_fill)
            .map(|offer| EventBody::VerifierAccept {
                fragment: self.fragment,
                attempt: last.attempt,
                offer: offer.event,
            })
            .collect()
    }

    fn finalize(&self, state: &TaskState) -> Option<EventBody> {
        if state.status != TaskStatus::Verified {
            return None;
        }
        let last = state.attempts.last()?;
        if self.mine.voted_finalize(self.fragment, last.attempt) {
            return None;
        }
        let result = last.result.as_ref()?.result;
        let mut approvals =
            self.ledger
                .approvals(self.job, self.fragment, last.attempt, self.now_ms);
        approvals.truncate(MAX_APPROVALS);
        if approvals.len() < self.required as usize {
            return None;
        }
        Some(EventBody::Finalize {
            fragment: self.fragment,
            attempt: last.attempt,
            certificate: Certificate {
                task: TaskRef::new(*self.job, self.fragment),
                attempt: last.attempt,
                result,
                approvals,
            },
        })
    }
}
