//! Le binaire `sil-raytracer-block` suit le protocole de `ExternalRenderer` : arguments, scène sur
//! l'entrée standard, exactement les octets de la bande sur la sortie standard, code 2 sinon.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sil_raytracer::render::render_rows;
use std::io::Write;
use std::process::{Command, Stdio};

const BLOCK: &str = env!("CARGO_BIN_EXE_sil-raytracer-block");

const SCENE: &str = "camera : { resolution = { width = 16 height = 10 } }
primitives : { sphere = { x = 5 y = 0 z = 0 radius = 1.5 } }
";

fn run(args: &[&str], stdin: &[u8]) -> (i32, Vec<u8>) {
    let mut child = Command::new(BLOCK)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary starts");
    child
        .stdin
        .take()
        .expect("stdin open")
        .write_all(stdin)
        .expect("scene written");
    let output = child.wait_with_output().expect("binary ends");
    (output.status.code().unwrap_or(-1), output.stdout)
}

#[test]
fn the_binary_writes_exactly_the_bytes_of_the_band() {
    let (code, pixels) = run(&["16", "10", "2", "6"], SCENE.as_bytes());
    assert_eq!(code, 0);
    assert_eq!(pixels, render_rows(SCENE, 16, 10, 2, 6).unwrap());
    assert_eq!(pixels.len(), 16 * 4 * 3);
}

#[test]
fn bad_arguments_or_a_bad_scene_exit_with_code_2_and_no_pixels() {
    let (code, pixels) = run(&["16", "10", "2"], SCENE.as_bytes());
    assert_eq!((code, pixels.len()), (2, 0));
    let (code, pixels) = run(&["16", "10", "0", "4"], b"render : {");
    assert_eq!((code, pixels.len()), (2, 0));
}
