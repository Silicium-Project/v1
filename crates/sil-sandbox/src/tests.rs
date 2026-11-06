use crate::{Confined, SandboxError, SandboxLimits, SandboxSpec, run};
use std::{path::PathBuf, time::Duration};

fn shell(script_windows: &str, script_unix: &str) -> (PathBuf, Vec<String>) {
    if cfg!(windows) {
        ("cmd".into(), vec!["/C".into(), script_windows.into()])
    } else {
        ("sh".into(), vec!["-c".into(), script_unix.into()])
    }
}

fn spec(dir: &std::path::Path, script_windows: &str, script_unix: &str) -> SandboxSpec {
    let (program, args) = shell(script_windows, script_unix);
    SandboxSpec {
        program,
        args,
        workdir: dir.to_path_buf(),
        env: Vec::new(),
        stdin: Vec::new(),
        limits: SandboxLimits {
            wall: Duration::from_secs(20),
            ..SandboxLimits::default()
        },
        isolation: None,
    }
}

#[test]
fn output_and_exit_code_are_captured() {
    let dir = tempfile::tempdir().unwrap();
    let outcome = run(&spec(dir.path(), "echo hello", "echo hello")).unwrap();
    assert!(outcome.success());
    assert!(String::from_utf8_lossy(&outcome.stdout).contains("hello"));
    let failing = run(&spec(dir.path(), "exit 3", "exit 3")).unwrap();
    assert_eq!(failing.code, Some(3));
}

#[test]
fn the_working_directory_is_the_one_given() {
    let dir = tempfile::tempdir().unwrap();
    let outcome = run(&spec(dir.path(), "cd", "pwd")).unwrap();
    let printed = String::from_utf8_lossy(&outcome.stdout).to_lowercase();
    let name = dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_lowercase();
    assert!(printed.contains(&name), "{printed}");
    let missing = SandboxSpec {
        workdir: dir.path().join("nope"),
        ..spec(dir.path(), "echo x", "echo x")
    };
    assert!(matches!(
        run(&missing),
        Err(SandboxError::WorkdirMissing(_))
    ));
}

#[test]
fn the_node_environment_is_not_inherited_only_the_allowlist() {
    let dir = tempfile::tempdir().unwrap();
    let mut with_env = spec(dir.path(), "set", "env");
    with_env.env.push(("SIL_TASK".into(), "42".into()));
    let outcome = run(&with_env).unwrap();
    let printed = String::from_utf8_lossy(&outcome.stdout).to_string();
    assert!(printed.contains("SIL_TASK=42"), "{printed}");
    let leaked = if cfg!(windows) {
        "USERPROFILE="
    } else {
        "HOME="
    };
    assert!(!printed.contains(leaked), "variable héritée : {printed}");
}

#[test]
fn stdin_is_forwarded() {
    let dir = tempfile::tempdir().unwrap();
    let program = if cfg!(windows) { "findstr" } else { "cat" };
    let args = if cfg!(windows) {
        vec!["^".to_string()]
    } else {
        Vec::new()
    };
    let outcome = run(&SandboxSpec {
        program: program.into(),
        args,
        stdin: b"through the pipe\r\n".to_vec(),
        ..spec(dir.path(), "", "")
    })
    .unwrap();
    assert!(String::from_utf8_lossy(&outcome.stdout).contains("through the pipe"));
}

#[test]
fn a_slow_process_is_killed_at_the_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let mut slow = spec(dir.path(), "ping -n 30 127.0.0.1 >nul", "sleep 30");
    // L'environnement n'est pas hérité : le PATH doit être donné explicitement.
    if cfg!(windows) {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        slow.env.push(("SystemRoot".into(), root.clone()));
        slow.env.push(("PATH".into(), format!(r"{root}\System32")));
    }
    slow.limits.wall = Duration::from_millis(400);
    let started = std::time::Instant::now();
    assert!(matches!(run(&slow), Err(SandboxError::Timeout)));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn runaway_output_kills_the_process() {
    let dir = tempfile::tempdir().unwrap();
    let mut noisy = spec(
        dir.path(),
        "for /L %i in (1,1,400000) do @echo aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "yes aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    noisy.limits.max_stdout = 10_000;
    assert!(matches!(
        run(&noisy),
        Err(SandboxError::OutputTooLarge { stream: "stdout" })
    ));
}

#[test]
fn a_missing_program_is_a_spawn_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = SandboxSpec {
        program: "definitely-not-a-real-program-sil".into(),
        ..spec(dir.path(), "", "")
    };
    assert!(matches!(run(&missing), Err(SandboxError::Spawn(_))));
}

#[test]
fn paths_are_confined_to_the_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = Confined::new(dir.path()).unwrap();
    let ok = root.resolve("scenes/a.cfg").unwrap();
    assert!(ok.starts_with(root.root()));
    for bad in [
        "",
        "../x",
        "a/../../b",
        "/etc/passwd",
        "C:/Windows",
        "a\\b",
        "a//b",
        "./a",
        "a/\0b",
        "..",
        ".",
    ] {
        assert!(root.resolve(bad).is_err(), "{bad:?} devrait être refusé");
    }
}

#[test]
fn symbolic_links_inside_the_root_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = Confined::new(dir.path()).unwrap();
    let link = root.root().join("link");
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(outside.path(), &link).is_ok();
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_dir(outside.path(), &link).is_ok();
    if !made {
        eprintln!("création de lien symbolique impossible ici (droits) : test ignoré");
        return;
    }
    assert!(matches!(
        root.resolve("link"),
        Err(SandboxError::Symlink(_))
    ));
    assert!(matches!(
        root.resolve("link/secret.txt"),
        Err(SandboxError::Symlink(_))
    ));
}

#[test]
fn without_an_explicit_path_the_program_cannot_find_other_programs() {
    if !cfg!(windows) {
        return; // sh retombe sur un PATH par défaut : comportement propre à Windows
    }
    let dir = tempfile::tempdir().unwrap();
    let outcome = run(&spec(dir.path(), "ping -n 1 127.0.0.1", "")).unwrap();
    assert_eq!(outcome.code, Some(1), "ping introuvable sans PATH");
}
