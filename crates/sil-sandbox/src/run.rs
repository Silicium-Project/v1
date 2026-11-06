use crate::{Isolation, Level, RunOutcome, SandboxError, SandboxSpec, capture};
use std::{
    io::Write,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

/// Attente maximale des lecteurs de flux après la fin du processus (un petit-enfant qui
/// garde le tube ouvert ne doit pas bloquer le nœud).
const DRAIN_WAIT: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(5);

/// Programme réellement lancé : le lanceur d'isolation (Linux), ou le programme lui-même.
fn command_line(spec: &SandboxSpec) -> Result<(PathBuf, Vec<String>), SandboxError> {
    let Some(isolation) = &spec.isolation else {
        return Ok((spec.program.clone(), spec.args.clone()));
    };
    if !cfg!(target_os = "linux") {
        return Ok((spec.program.clone(), spec.args.clone()));
    }
    let launcher = isolation.launcher.clone().ok_or_else(|| {
        SandboxError::IsolationUnavailable("lanceur sil-sandbox-launch introuvable".into())
    })?;
    // Le lanceur reçoit la politique en JSON, puis `--`, puis le programme et ses arguments.
    let policy =
        serde_json::to_string(isolation).map_err(|e| SandboxError::Isolation(e.to_string()))?;
    let mut args = vec![
        policy,
        "--".into(),
        spec.program.to_string_lossy().into_owned(),
    ];
    args.extend(spec.args.iter().cloned());
    Ok((launcher, args))
}

/// Refuse avant tout lancement une isolation que cette plateforme ne sait pas appliquer.
fn check_platform(isolation: &Isolation) -> Result<(), SandboxError> {
    if !cfg!(target_os = "linux") && isolation.level == Level::Strict {
        return Err(SandboxError::IsolationUnavailable(
            "sous Windows, l'isolation stricte n'est pas disponible (voir sandbox.md)".into(),
        ));
    }
    Ok(())
}

fn kill(child: &mut Child) {
    kill_group(child);
    let _ = child.kill();
    let _ = child.wait();
}

/// Tue le groupe de processus du programme (il a été lancé dans son propre groupe) : ses enfants
/// meurent avec lui. Un identifiant nul ou invalide n'est jamais utilisé : `killpg(0)` viserait
/// le nœud lui-même.
#[cfg(unix)]
fn kill_group(child: &Child) {
    use nix::{
        sys::signal::{Signal, killpg},
        unistd::Pid,
    };
    if let Ok(pid) = i32::try_from(child.id())
        && pid > 0
    {
        let _ = killpg(Pid::from_raw(pid), Signal::SIGKILL);
    }
}

#[cfg(not(unix))]
fn kill_group(_child: &Child) {}

/// Lance le programme, attend sa fin dans les limites, rend sa sortie.
pub fn run(spec: &SandboxSpec) -> Result<RunOutcome, SandboxError> {
    if !spec.workdir.is_dir() {
        return Err(SandboxError::WorkdirMissing(spec.workdir.clone()));
    }
    if let Some(isolation) = &spec.isolation {
        check_platform(isolation)?;
    }
    let (program, args) = command_line(spec)?;
    let mut command = Command::new(&program);
    command
        .args(&args)
        .current_dir(&spec.workdir)
        .env_clear()
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(SandboxError::Spawn)?;
    let started = Instant::now();
    // Windows : le Job Object doit contenir le processus avant qu'il ne fasse quoi que ce soit ;
    // sa fermeture (fin de cette fonction) tue l'arbre.
    #[cfg(windows)]
    let _tree = match &spec.isolation {
        Some(_) => match crate::windows::guard(&child) {
            Ok(guard) => Some(guard),
            Err(error) => {
                kill(&mut child);
                return Err(SandboxError::Isolation(error));
            }
        },
        None => None,
    };

    if let Some(mut stdin) = child.stdin.take() {
        let input = spec.stdin.clone();
        thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    let (out_flag, err_flag) = (
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let stdout = child
        .stdout
        .take()
        .map(|s| capture::spawn(s, spec.limits.max_stdout, out_flag.clone()));
    let stderr = child
        .stderr
        .take()
        .map(|s| capture::spawn(s, spec.limits.max_stderr, err_flag.clone()));

    let status = loop {
        if out_flag.load(Ordering::SeqCst) {
            kill(&mut child);
            return Err(SandboxError::OutputTooLarge { stream: "stdout" });
        }
        if err_flag.load(Ordering::SeqCst) {
            kill(&mut child);
            return Err(SandboxError::OutputTooLarge { stream: "stderr" });
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                kill(&mut child);
                return Err(SandboxError::Io(error.to_string()));
            }
        }
        if started.elapsed() > spec.limits.wall {
            kill(&mut child);
            return Err(SandboxError::Timeout);
        }
        thread::sleep(POLL);
    };
    // Le programme est terminé : ses enfants restants ne doivent pas survivre.
    kill_group(&child);

    let drain = |receiver: Option<std::sync::mpsc::Receiver<Vec<u8>>>| {
        receiver.map_or_else(
            || Ok(Vec::new()),
            |r| {
                r.recv_timeout(DRAIN_WAIT)
                    .map_err(|_| SandboxError::Io("flux non terminé".into()))
            },
        )
    };
    let stdout = drain(stdout)?;
    let stderr = drain(stderr)?;
    // Un dépassement détecté pendant le drainage ne doit pas passer pour un succès.
    if out_flag.load(Ordering::SeqCst) {
        return Err(SandboxError::OutputTooLarge { stream: "stdout" });
    }
    if err_flag.load(Ordering::SeqCst) {
        return Err(SandboxError::OutputTooLarge { stream: "stderr" });
    }
    Ok(RunOutcome {
        code: status.code(),
        stdout,
        stderr,
        elapsed: started.elapsed(),
    })
}
