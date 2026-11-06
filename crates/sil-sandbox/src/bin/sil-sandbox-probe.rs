//! Programme de test de l'isolation. Chaque mode tente une action et rend un code de sortie :
//! `0` l'action a réussi (donc non isolé), `10` réseau refusé, `11` écriture refusée,
//! `12` lecture refusée, `13` allocation refusée, `14` création de processus refusée, `2` usage.
//! Modes : `net <port>`, `write <chemin>`, `read <chemin>`, `alloc <octets>`, `spawn`, `slow`.

use std::{net::TcpStream, process::ExitCode, time::Duration};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code: u8 = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["net", port] => match port.parse::<u16>() {
            Ok(port) => match TcpStream::connect(("127.0.0.1", port)) {
                Ok(_) => 0,
                Err(_) => 10,
            },
            Err(_) => 2,
        },
        ["write", path] => match std::fs::write(path, b"probe") {
            Ok(()) => 0,
            Err(_) => 11,
        },
        ["read", path] => match std::fs::read(path) {
            Ok(_) => 0,
            Err(_) => 12,
        },
        ["alloc", bytes] => match bytes.parse::<usize>() {
            Ok(size) => {
                let mut buffer: Vec<u8> = Vec::new();
                match buffer.try_reserve_exact(size) {
                    Ok(()) => {
                        buffer.resize(size, 1);
                        0
                    }
                    Err(_) => 13,
                }
            }
            Err(_) => 2,
        },
        ["spawn"] => spawn_child(),
        ["slow"] => {
            std::thread::sleep(Duration::from_secs(60));
            0
        }
        _ => 2,
    };
    ExitCode::from(code)
}

/// Lance une copie de soi-même qui dort, et attend : sert à tester qu'un délai tue l'arbre.
fn spawn_child() -> u8 {
    let Ok(me) = std::env::current_exe() else {
        return 2;
    };
    match std::process::Command::new(me).arg("slow").spawn() {
        Ok(mut child) => {
            let _ = child.wait();
            0
        }
        Err(_) => 14,
    }
}
