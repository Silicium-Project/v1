//! Tests de la politique de réclamation.

use crate::claim_policy::*;
use std::collections::BTreeMap;

mod tests {
    use super::*;

    #[test]
    fn a_new_node_runs_its_start_count_of_tasks() {
        assert_eq!(allowed_parallel(1, 15), START_PARALLEL);
    }

    #[test]
    fn reputation_raises_the_cap_up_to_the_cores() {
        assert_eq!(allowed_parallel(6, 15), 6);
        assert_eq!(allowed_parallel(8, 3), 3);
    }

    #[test]
    fn a_banned_worker_gets_no_task() {
        assert_eq!(allowed_parallel(0, 15), 0);
    }

    #[test]
    fn with_work_waiting_and_other_workers_each_node_takes_one_task() {
        assert_eq!(claim_cap(5, 2, 3), 1);
    }

    #[test]
    fn a_lone_worker_keeps_its_parallelism() {
        assert_eq!(claim_cap(5, 1, 3), 3);
    }

    #[test]
    fn nothing_waiting_means_the_normal_cap() {
        assert_eq!(claim_cap(0, 2, 3), 3);
    }

    #[test]
    fn an_expired_lease_no_longer_counts_as_a_running_task() {
        assert!(lease_held(2_000, 1_000));
        assert!(!lease_held(1_000, 1_000));
        assert!(!lease_held(500, 1_000));
    }

    #[test]
    fn a_banned_worker_stays_at_zero() {
        assert_eq!(claim_cap(5, 2, 0), 0);
    }

    #[test]
    fn a_worker_is_still_counted_after_three_lost_presence_messages() {
        // Présence toutes les 15 s : trois messages perdus, la dernière reçue il y a 50 s.
        let peer = sil_net::PeerId::random();
        let presence = sil_net::Presence {
            version: 1,
            roles: sil_net::ROLE_COMPUTE,
            load: 0,
            workloads: Vec::new(),
        };
        let mut book = BTreeMap::new();
        book.insert(peer, (presence, 1_000_000));
        assert_eq!(compute_peers_alive(&book, 1_050_000), 1);
        // Au-delà de la fenêtre, le pair est bien compté comme parti.
        assert_eq!(compute_peers_alive(&book, 1_000_000 + PRESENCE_FRESH_MS), 0);
    }

    #[test]
    fn slower_needs_a_fifty_percent_margin() {
        assert!(!slower(100, 100));
        assert!(!slower(150, 100));
        assert!(slower(151, 100));
    }

    #[test]
    fn speed_needs_a_few_samples() {
        assert_eq!(speed_from(&[(0, 10)]), None);
    }

    #[test]
    fn speed_is_the_median_of_the_recent_durations() {
        // Durées 10, 30, 60 ms : médiane 30 ; dernier claim à 200 ms.
        assert_eq!(
            speed_from(&[(0, 10), (100, 130), (200, 260)]),
            Some((30, 200))
        );
    }

    #[test]
    fn old_slow_samples_stop_counting() {
        let mut samples: Vec<(u64, u64)> =
            (0..4).map(|i| (i * 10_000, i * 10_000 + 90_000)).collect();
        samples.extend((4..12).map(|i| (i * 10_000, i * 10_000 + 1_000)));
        let (median, _) = speed_from(&samples).expect("assez de mesures");
        assert_eq!(median, 1_000);
    }
}
