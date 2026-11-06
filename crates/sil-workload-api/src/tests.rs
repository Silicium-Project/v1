use crate::{Evidence, InputReader, MemoryInput, Output, Registry, Workload};
use sil_types::Hash32;
use std::sync::Arc;

#[test]
fn memory_input_is_readable_by_range_and_bounds_are_checked() {
    let input = MemoryInput(b"hello world");
    assert_eq!(input.len(), 11);
    assert_eq!(input.read(6, 5).unwrap(), b"world");
    assert!(input.read(6, 6).is_err());
    assert!(input.read(u64::MAX, 1).is_err());
    assert!(MemoryInput(&[]).is_empty());
}

#[test]
fn output_hashes_depend_on_content_and_on_milestones() {
    let a = Output {
        result: vec![1, 2, 3],
        milestones: vec![Hash32::from_bytes([1; 32])],
    };
    let mut b = a.clone();
    assert_eq!(a.result_hash(), b.result_hash());
    b.milestones.push(Hash32::from_bytes([2; 32]));
    assert_eq!(a.result_hash(), b.result_hash());
    assert_ne!(a.milestones_root(), b.milestones_root());
}

#[test]
fn evidence_is_canonical_and_bounded() {
    let long = "x".repeat(500);
    let evidence = Evidence::new(
        3,
        Hash32::from_bytes([1; 32]),
        Hash32::from_bytes([2; 32]),
        &long,
    );
    assert_eq!(evidence.reason.len(), 100);
    assert_eq!(evidence.hash(), evidence.clone().hash());
    let other = Evidence::new(4, evidence.committed, evidence.recomputed, "x");
    assert_ne!(evidence.hash(), other.hash());
}

#[test]
fn an_empty_registry_knows_no_workload() {
    let registry = Registry::new();
    assert!(registry.get("calcul/sha256").is_none());
    assert!(registry.ids().is_empty());
    let _: Option<Arc<dyn Workload>> = registry.get("x");
}
