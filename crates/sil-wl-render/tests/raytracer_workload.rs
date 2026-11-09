//! Le workload `rendu/raytracer` avec le vrai moteur (`sil-raytracer`) : les fragments se
//! recombinent en l'image entière, un fragment honnête est approuvé, un fragment falsifié est rejeté.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sil_raytracer::render::render_rows;
use sil_wl_render::{BlockRenderer, RenderWorkload};
use sil_workload_api::{MemoryInput, Verdict, Workload, WorkloadError};

const SCENE: &str = "render : { samples = 1 }
camera : {
  resolution = { width = 16 height = 10 }
  position = { x = 0 y = 0 z = 0 }
  look_at = { x = 5 y = 0 z = 0 }
  fieldOfView = 55
}
primitives : {
  plane = { z = -1 type = diffuse }
  sphere = { x = 5 y = 0 z = 0 radius = 1.5 color = { r = 255 g = 80 b = 40 } }
}
lights : { point = { x = 2 y = 3 z = 4 intensity = 450 } }
";

/// Le moteur de production : le raytracer, en processus, à la demande de la bande.
struct Raytracer;

impl BlockRenderer for Raytracer {
    fn render(
        &self,
        scene: &[u8],
        width: u32,
        height: u32,
        y0: u32,
        y1: u32,
    ) -> Result<Vec<u8>, WorkloadError> {
        let text = std::str::from_utf8(scene)
            .map_err(|_| WorkloadError::BadInput("scène non UTF-8".into()))?;
        render_rows(text, width, height, y0, y1)
            .map_err(|error| WorkloadError::Execution(error.to_string()))
    }
}

/// En-tête (largeur puis hauteur, u32 little-endian) suivi de la scène : l'entrée du job.
fn job_input(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(SCENE.as_bytes());
    bytes
}

#[test]
fn the_fragments_of_a_scene_recombine_into_the_whole_image() {
    let bytes = job_input(16, 10);
    let input = MemoryInput(&bytes);
    let workload = RenderWorkload::new(Raytracer);
    let specs = workload.split(&input, 2).unwrap();
    let outputs: Vec<_> = specs
        .iter()
        .map(|spec| workload.execute(&input, spec).unwrap())
        .collect();
    let image = workload.combine(&outputs).unwrap();
    assert_eq!(image, render_rows(SCENE, 16, 10, 0, 10).unwrap());
}

#[test]
fn an_honest_fragment_is_approved_by_recomputing_its_blocks() {
    let bytes = job_input(16, 10);
    let input = MemoryInput(&bytes);
    let workload = RenderWorkload::new(Raytracer);
    for spec in workload.split(&input, 2).unwrap() {
        let output = workload.execute(&input, &spec).unwrap();
        let blocks = workload.block_count(&spec).unwrap();
        let every_block: Vec<u32> = (0..blocks).collect();
        let verdict = workload
            .verify(&input, &spec, &output, &every_block)
            .unwrap();
        assert_eq!(verdict, Verdict::Approve);
    }
}

#[test]
fn a_falsified_fragment_is_rejected_with_its_proof() {
    let bytes = job_input(16, 10);
    let input = MemoryInput(&bytes);
    let workload = RenderWorkload::new(Raytracer);
    let spec = workload.split(&input, 2).unwrap().remove(0);
    let mut output = workload.execute(&input, &spec).unwrap();
    // Un pixel altéré après l'engagement : le bloc ne correspond plus à son hash.
    output.result[0] ^= 0x01;
    let blocks = workload.block_count(&spec).unwrap();
    let every_block: Vec<u32> = (0..blocks).collect();
    let verdict = workload
        .verify(&input, &spec, &output, &every_block)
        .unwrap();
    assert!(matches!(verdict, Verdict::Reject(_)), "{verdict:?}");
}
