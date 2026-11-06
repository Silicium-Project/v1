use crate::Sha256Workload;
use sil_crypto::sha256;
use sil_workload_api::{FragmentSpec, MemoryInput, Verdict, Workload, WorkloadError};

fn data(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 + 3) as u8).collect()
}

#[test]
fn fragments_tile_the_input_exactly() {
    let bytes = data(1003);
    let input = MemoryInput(&bytes);
    let specs = Sha256Workload.split(&input, 4).unwrap();
    assert_eq!(specs.len(), 4);
    let mut total = Vec::new();
    for spec in &specs {
        total.extend(Sha256Workload.execute(&input, spec).unwrap().result);
    }
    assert_eq!(total.len(), 4 * 32);
    // Chaque fragment est le hash de sa plage, et les plages couvrent l'entrée (1003 = 251*3+250).
    let first = sha256(&bytes[..251]);
    assert_eq!(&total[..32], first.as_bytes());
}

#[test]
fn tiny_and_empty_inputs_never_produce_empty_fragments_beyond_need() {
    let empty = MemoryInput(&[]);
    assert_eq!(Sha256Workload.split(&empty, 8).unwrap().len(), 1);
    let two = MemoryInput(&[1, 2]);
    assert_eq!(Sha256Workload.split(&two, 8).unwrap().len(), 2);
    assert!(Sha256Workload.split(&two, 0).is_err());
    assert!(Sha256Workload.split(&two, 5000).is_err());
}

#[test]
fn a_correct_result_is_approved_and_a_wrong_one_is_rejected_with_evidence() {
    let bytes = data(5000);
    let input = MemoryInput(&bytes);
    let spec = &Sha256Workload.split(&input, 2).unwrap()[1];
    let honest = Sha256Workload.execute(&input, spec).unwrap();
    assert_eq!(
        Sha256Workload.verify(&input, spec, &honest, &[]).unwrap(),
        Verdict::Approve
    );

    let mut forged = honest.clone();
    forged.result[0] ^= 1;
    match Sha256Workload.verify(&input, spec, &forged, &[]).unwrap() {
        Verdict::Reject(evidence) => {
            assert_eq!(evidence.recomputed.as_bytes(), honest.result.as_slice());
            assert_ne!(evidence.committed, evidence.recomputed);
        }
        Verdict::Approve => panic!("un faux hash a été approuvé"),
    }

    let short = sil_workload_api::Output {
        result: vec![1, 2, 3],
        milestones: Vec::new(),
    };
    assert!(matches!(
        Sha256Workload.verify(&input, spec, &short, &[]).unwrap(),
        Verdict::Reject(_)
    ));
}

#[test]
fn specs_outside_the_input_or_garbled_are_refused() {
    let bytes = data(100);
    let input = MemoryInput(&bytes);
    let garbled = FragmentSpec {
        index: 0,
        params: vec![1, 2, 3],
    };
    assert!(matches!(
        Sha256Workload.execute(&input, &garbled),
        Err(WorkloadError::BadSpec(_))
    ));
    let spec = &Sha256Workload.split(&MemoryInput(&data(500)), 1).unwrap()[0];
    assert!(
        Sha256Workload.execute(&input, spec).is_err(),
        "plage > entrée"
    );
}

#[test]
fn the_job_result_is_deterministic() {
    let bytes = data(4096);
    let input = MemoryInput(&bytes);
    let run = || {
        let specs = Sha256Workload.split(&input, 3).unwrap();
        let outputs: Vec<_> = specs
            .iter()
            .map(|s| Sha256Workload.execute(&input, s).unwrap())
            .collect();
        Sha256Workload.combine(&outputs).unwrap()
    };
    assert_eq!(run(), run());
}
