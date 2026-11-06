//! Isolation : chaque protection est testée contre la sonde `sil-sandbox-probe`, avec un témoin
//! sans isolation pour prouver que la sonde peut réellement faire l'action interdite.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sil_sandbox::{Isolation, SandboxError, SandboxLimits, SandboxSpec, run};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const PROBE: &str = env!("CARGO_BIN_EXE_sil-sandbox-probe");
const LAUNCH: &str = env!("CARGO_BIN_EXE_sil-sandbox-launch");

fn spec(args: &[String], workdir: &Path, isolated: bool) -> SandboxSpec {
    let program = PathBuf::from(PROBE);
    let isolation = isolated.then(|| {
        let mut policy = Isolation::for_program(&program, workdir, Some(PathBuf::from(LAUNCH)));
        policy.memory_bytes = 1 << 30;
        policy
    });
    SandboxSpec {
        program,
        args: args.to_vec(),
        workdir: workdir.to_path_buf(),
        env: Vec::new(),
        stdin: Vec::new(),
        limits: SandboxLimits {
            wall: Duration::from_secs(20),
            ..SandboxLimits::default()
        },
        isolation,
    }
}

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

#[cfg(target_os = "linux")]
fn code(spec: &SandboxSpec) -> Option<i32> {
    run(spec).expect("le programme tourne").code
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn without_isolation_the_probe_reaches_the_network() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port().to_string();
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            code(&spec(&args(&["net", &port]), dir.path(), false)),
            Some(0)
        );
    }

    #[test]
    fn isolated_probe_cannot_open_a_network_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port().to_string();
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            code(&spec(&args(&["net", &port]), dir.path(), true)),
            Some(10)
        );
    }

    #[test]
    fn isolated_probe_cannot_write_outside_the_workdir() {
        let workdir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("written.txt");
        let target_text = target.display().to_string();

        assert_eq!(
            code(&spec(
                &args(&["write", &target_text]),
                workdir.path(),
                false
            )),
            Some(0)
        );
        assert!(
            target.exists(),
            "témoin : sans isolation, l'écriture réussit"
        );
        std::fs::remove_file(&target).unwrap();

        assert_eq!(
            code(&spec(&args(&["write", &target_text]), workdir.path(), true)),
            Some(11)
        );
        assert!(
            !target.exists(),
            "rien n'a été écrit hors du dossier de travail"
        );
    }

    #[test]
    fn isolated_probe_can_write_inside_the_workdir() {
        let workdir = tempfile::tempdir().unwrap();
        let inside = workdir.path().join("inside.txt");
        let inside_text = inside.display().to_string();
        assert_eq!(
            code(&spec(&args(&["write", &inside_text]), workdir.path(), true)),
            Some(0)
        );
        assert!(inside.exists());
    }

    #[test]
    fn isolated_probe_cannot_read_files_outside_the_allowed_paths() {
        let workdir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        std::fs::write(&secret, b"cle").unwrap();
        let secret_text = secret.display().to_string();

        assert_eq!(
            code(&spec(&args(&["read", &secret_text]), workdir.path(), false)),
            Some(0)
        );
        assert_eq!(
            code(&spec(&args(&["read", &secret_text]), workdir.path(), true)),
            Some(12)
        );
    }

    #[test]
    fn isolated_probe_cannot_allocate_beyond_the_memory_limit() {
        let workdir = tempfile::tempdir().unwrap();
        let four_gib = (4u64 << 30).to_string();
        assert_eq!(
            code(&spec(&args(&["alloc", &four_gib]), workdir.path(), true)),
            Some(13)
        );
    }

    #[test]
    fn strict_isolation_without_a_launcher_is_refused_before_running() {
        let workdir = tempfile::tempdir().unwrap();
        let mut spec = spec(&args(&["slow"]), workdir.path(), true);
        spec.isolation.as_mut().unwrap().launcher = None;
        let started = Instant::now();
        assert!(matches!(
            run(&spec),
            Err(SandboxError::IsolationUnavailable(_))
        ));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "rien n'a été lancé"
        );
    }

    #[test]
    fn a_timeout_kills_the_program_and_its_children() {
        let workdir = tempfile::tempdir().unwrap();
        let mut spec = spec(&args(&["spawn"]), workdir.path(), false);
        spec.limits.wall = Duration::from_secs(1);
        let started = Instant::now();
        assert!(matches!(run(&spec), Err(SandboxError::Timeout)));
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "l'enfant qui dort 60 s est mort avec son parent"
        );
    }

    #[test]
    fn an_isolated_timeout_kills_the_program_and_its_children() {
        let workdir = tempfile::tempdir().unwrap();
        let mut spec = spec(&args(&["spawn"]), workdir.path(), true);
        spec.limits.wall = Duration::from_secs(1);
        let started = Instant::now();
        assert!(matches!(run(&spec), Err(SandboxError::Timeout)));
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use sil_sandbox::Level;

    #[test]
    fn strict_isolation_is_refused_on_windows() {
        let workdir = tempfile::tempdir().unwrap();
        let spec = spec(&args(&["slow"]), workdir.path(), true);
        assert!(matches!(
            run(&spec),
            Err(SandboxError::IsolationUnavailable(_))
        ));
    }

    #[test]
    fn best_effort_timeout_kills_the_job_tree() {
        let workdir = tempfile::tempdir().unwrap();
        let mut spec = spec(&args(&["spawn"]), workdir.path(), true);
        spec.isolation.as_mut().unwrap().level = Level::BestEffort;
        spec.limits.wall = Duration::from_secs(1);
        let started = Instant::now();
        assert!(matches!(run(&spec), Err(SandboxError::Timeout)));
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
