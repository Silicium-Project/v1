use crate::*;
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_types::{Hash32, JobId, TaskRef};

fn verifier(n: u8) -> SigningIdentity {
    MasterSeed::from_bytes([n; 32]).derive(Purpose::Ledger)
}

fn task() -> TaskRef {
    TaskRef::new(JobId::from_bytes([1; 32]), 2)
}

fn result() -> Hash32 {
    Hash32::from_bytes([3; 32])
}

#[test]
fn the_seed_is_deterministic_and_checkable_by_anyone() {
    let id = verifier(1);
    let (seed, signature) = SamplingSeed::draw(&id, &task(), 1, &result());
    let (again, _) = SamplingSeed::draw(&id, &task(), 1, &result());
    assert_eq!(seed, again, "le vérificateur ne peut pas retirer");
    let checked = SamplingSeed::verify(&id.public(), &task(), 1, &result(), &signature).unwrap();
    assert_eq!(checked, seed);
    assert!(
        SamplingSeed::verify(&id.public(), &task(), 2, &result(), &signature).is_err(),
        "une signature d'une autre tentative n'est pas une graine valide"
    );
}

#[test]
fn each_verifier_and_each_result_gets_its_own_sample() {
    let (a, _) = SamplingSeed::draw(&verifier(1), &task(), 1, &result());
    let (b, _) = SamplingSeed::draw(&verifier(2), &task(), 1, &result());
    let (c, _) = SamplingSeed::draw(&verifier(1), &task(), 1, &Hash32::from_bytes([4; 32]));
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(sample_indices(&a, 1000, 20), sample_indices(&b, 1000, 20));
}

#[test]
fn indices_are_distinct_sorted_in_range_and_reproducible() {
    let (seed, _) = SamplingSeed::draw(&verifier(1), &task(), 1, &result());
    let sample = sample_indices(&seed, 100, 30);
    assert_eq!(sample.len(), 30);
    assert!(sample.windows(2).all(|w| w[0] < w[1]));
    assert!(sample.iter().all(|i| *i < 100));
    assert_eq!(sample, sample_indices(&seed, 100, 30));
    assert_eq!(
        sample_indices(&seed, 5, 50),
        vec![0, 1, 2, 3, 4],
        "plafonné au total"
    );
    assert!(sample_indices(&seed, 0, 5).is_empty());
}

#[test]
fn detection_probability_matches_known_values() {
    // Un bloc falsifié sur 100, 8 blocs contrôlés (règle de V0) : 8 %.
    let v0 = detection_probability(100, 1, 8);
    assert!((v0 - 0.08).abs() < 1e-9, "{v0}");
    assert_eq!(detection_probability(100, 0, 50), 0.0);
    assert_eq!(
        detection_probability(10, 6, 5),
        1.0,
        "au moins un mauvais est forcément vu"
    );
    assert!(detection_probability(100, 20, 20) > 0.98);
}

#[test]
fn samples_needed_reaches_the_target_and_is_minimal() {
    for (total, fraction) in [(100, 0.2), (1000, 0.2), (100, 0.05), (10, 0.5)] {
        let k = samples_needed(total, fraction, 0.99);
        let bad = ((f64::from(total) * fraction).ceil() as u32).max(1);
        assert!(
            detection_probability(total, bad, k) >= 0.99,
            "{total} {fraction}"
        );
        assert!(k == 1 || detection_probability(total, bad, k - 1) < 0.99);
    }
    assert_eq!(samples_needed(0, 0.2, 0.99), 0);
    assert!(
        samples_needed(1000, 0.2, 0.99) <= 25,
        "indépendant de la taille"
    );
}

#[test]
fn a_cheater_is_caught_at_the_promised_rate_over_many_seeds() {
    let total = 100;
    let bad: Vec<u32> = (0..20).collect();
    let k = samples_needed(total, 0.2, 0.99);
    let caught = (0..400u32)
        .filter(|n| {
            let id = verifier((n % 250) as u8 + 1);
            let result = Hash32::from_bytes([(n / 250) as u8; 32]);
            let (seed, _) = SamplingSeed::draw(&id, &task(), 1, &result);
            sample_indices(&seed, total, k)
                .iter()
                .any(|i| bad.contains(i))
        })
        .count();
    assert!(
        caught >= 390,
        "{caught}/400 détectés pour une cible de 99 %"
    );
}
