//! Engine binary, three modes, the scene always on the standard input and nothing else read:
//!
//! - `sil-raytracer-block <largeur> <hauteur> <y0> <y1>`: the RGB24 pixels of the band on the
//!   standard output, and nothing else (the protocol of `sil-wl-render::ExternalRenderer`);
//! - `sil-raytracer-block check`: the JSON report of the scene checker on the standard output;
//! - `sil-raytracer-block obj <matière> <taille> <y|z>`: the `mesh` block of an OBJ model (the
//!   standard input) on the standard output, centred, scaled to `taille`, y-up or z-up.
//!
//! Exit code 2 on bad arguments or invalid input, with the reason on the standard error. The binary
//! opens no file and no socket: the sandbox that runs it gives it nothing more. It also builds for
//! `wasm32-wasip1`, where the browser runs the same code for previews and checks.

use sil_raytracer::check::check_scene;
use sil_raytracer::obj;
use sil_raytracer::render::{render_bands, render_rows};
use std::io::{self, Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(output) => match io::stdout()
            .lock()
            .write_all(&output)
            .and_then(|()| io::stdout().flush())
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(&error.to_string()),
        },
        Err(message) => fail(&message),
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("sil-raytracer-block: {message}");
    ExitCode::from(2)
}

fn run() -> Result<Vec<u8>, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => check(),
        Some("obj") if args.len() == 4 => import(&args[1], &args[2], &args[3]),
        Some("bands") => bands(&args[1..]),
        _ => render(&args),
    }
}

/// Reads the whole standard input as UTF-8 text.
fn stdin_text() -> Result<String, String> {
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .map_err(|error| error.to_string())?;
    String::from_utf8(input).map_err(|_| "l'entrée n'est pas du texte UTF-8".to_string())
}

fn check() -> Result<Vec<u8>, String> {
    let report = check_scene(&stdin_text()?);
    serde_json::to_vec(&report).map_err(|error| error.to_string())
}

fn import(material: &str, size: &str, up: &str) -> Result<Vec<u8>, String> {
    let size: f64 = size
        .parse()
        .map_err(|_| "taille non numérique".to_string())?;
    let mut model = obj::parse(&stdin_text()?)?;
    match up {
        "y" => model.y_up_to_z_up(),
        "z" => {}
        _ => return Err("axe vertical : y ou z".into()),
    }
    model.fit(size);
    Ok(model.to_block(material, "imported OBJ").into_bytes())
}

/// `bands <largeur> <hauteur> <y0>-<y1> ...` : the bands one after another, in the order given,
/// with the scene read once. Same bytes as one `<largeur> <hauteur> <y0> <y1>` call per band.
fn bands(args: &[String]) -> Result<Vec<u8>, String> {
    if args.len() < 3 {
        return Err("arguments : bands <largeur> <hauteur> <y0>-<y1> ...".into());
    }
    let width = args[0]
        .parse::<u32>()
        .map_err(|_| "largeur non numérique".to_string())?;
    let height = args[1]
        .parse::<u32>()
        .map_err(|_| "hauteur non numérique".to_string())?;
    let mut list = Vec::with_capacity(args.len() - 2);
    for spec in &args[2..] {
        let (first, last) = spec
            .split_once('-')
            .ok_or_else(|| format!("bande {spec} : y0-y1 attendu"))?;
        let y0 = first
            .parse::<u32>()
            .map_err(|_| format!("bande {spec} : y0 non numérique"))?;
        let y1 = last
            .parse::<u32>()
            .map_err(|_| format!("bande {spec} : y1 non numérique"))?;
        list.push((y0, y1));
    }
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let text = stdin_text()?;
    render_bands(&text, width, height, &list, threads).map_err(|error| error.to_string())
}

fn render(args: &[String]) -> Result<Vec<u8>, String> {
    if args.len() != 4 {
        return Err(
            "arguments : <largeur> <hauteur> <y0> <y1> | check | obj <matière> <taille> <y|z>"
                .into(),
        );
    }
    let number = |index: usize| {
        args[index]
            .parse::<u32>()
            .map_err(|_| format!("argument {index} non numérique"))
    };
    let (width, height, y0, y1) = (number(0)?, number(1)?, number(2)?, number(3)?);
    let text = stdin_text()?;
    render_rows(&text, width, height, y0, y1).map_err(|error| error.to_string())
}
