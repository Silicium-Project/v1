//! Lanceur d'isolation : `sil-sandbox-launch <politique JSON> -- <programme> <args...>`.
//! Applique la politique au processus courant, puis remplace ce processus par le programme.
//! Réservé à `sil-sandbox` : ne pas l'appeler à la main.

use sil_sandbox::{Isolation, apply_isolation};
use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("sil-sandbox-launch : {}", launch());
    ExitCode::from(125)
}

/// Ne revient qu'en cas d'échec : en cas de succès, `exec` remplace le processus.
fn launch() -> String {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [policy, separator, program, rest @ ..] = args.as_slice() else {
        return "arguments : <politique JSON> -- <programme> <args...>".into();
    };
    if separator != "--" {
        return "séparateur -- attendu".into();
    }
    let policy: Isolation = match serde_json::from_str(policy) {
        Ok(policy) => policy,
        Err(error) => return format!("politique illisible : {error}"),
    };
    if let Err(message) = apply_isolation(&policy) {
        return message;
    }
    exec(program, rest)
}

#[cfg(unix)]
fn exec(program: &str, args: &[String]) -> String {
    use std::os::unix::process::CommandExt;
    let error = std::process::Command::new(program).args(args).exec();
    format!("exec {program} : {error}")
}

#[cfg(not(unix))]
fn exec(_program: &str, _args: &[String]) -> String {
    "lanceur absent sur cette plateforme".into()
}
