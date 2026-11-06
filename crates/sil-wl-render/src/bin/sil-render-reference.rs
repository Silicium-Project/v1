//! Moteur de référence en processus séparé, au protocole de `ExternalRenderer` :
//! `sil-render-reference <largeur> <hauteur> <y0> <y1> [mode]`, la scène sur l'entrée standard,
//! les pixels RGB24 sur la sortie standard. Code 2 si les arguments sont mauvais.
//!
//! `mode` ne sert qu'aux tests du nœud : `slow` (dépasse le délai), `flood` (sortie géante),
//! `short` (pixels manquants). Sans mode : rendu normal.

use sil_wl_render::reference_pixels;
use std::{
    io::{self, Read, Write},
    process::ExitCode,
    time::Duration,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("sil-render-reference : {message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        return Err("arguments : <largeur> <hauteur> <y0> <y1> [mode]".into());
    }
    let number = |index: usize| {
        args[index]
            .parse::<u32>()
            .map_err(|_| format!("argument {index} non numérique"))
    };
    let (width, height, y0, y1) = (number(0)?, number(1)?, number(2)?, number(3)?);
    if width == 0 || y0 >= y1 || y1 > height {
        return Err("bande hors de l'image".into());
    }
    let mut scene = Vec::new();
    io::stdin()
        .read_to_end(&mut scene)
        .map_err(|e| e.to_string())?;
    let mut pixels = reference_pixels(&scene, width, y0, y1);
    let mut out = io::stdout().lock();
    match args.get(4).map(String::as_str) {
        // Sondes d'isolation (tests) : code 10 réseau refusé, 11 écriture refusée, 13 allocation
        // refusée. Un code 0 signifie que l'action a réussi.
        Some("net") => {
            std::process::exit(if std::net::TcpStream::connect("127.0.0.1:9").is_err() {
                10
            } else {
                0
            })
        }
        Some("write") => {
            std::process::exit(if std::fs::write("/sil-probe.txt", b"probe").is_err() {
                11
            } else {
                0
            })
        }
        Some("alloc") => {
            let mut buffer: Vec<u8> = Vec::new();
            if buffer.try_reserve_exact(1 << 30).is_err() {
                std::process::exit(13);
            }
        }
        Some("slow") => std::thread::sleep(Duration::from_secs(30)),
        // Code 12 : l'hôte a refusé la sortie (au-delà de sa capacité).
        Some("flood") => {
            let chunk = vec![0u8; 1 << 20];
            loop {
                if out.write_all(&chunk).is_err() {
                    std::process::exit(12);
                }
            }
        }
        Some("short") => pixels.truncate(pixels.len().saturating_sub(3)),
        _ => {}
    }
    out.write_all(&pixels).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}
