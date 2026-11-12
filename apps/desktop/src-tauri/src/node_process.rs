//! Runs the Silicium node as a worker and verifier, detached from this window, and reads its report.
//!
//! The node is a separate program (`silicium-node`). Once activated it keeps running when this
//! window is closed: it is stopped only by `stop` (the user's "Désactiver ce PC"), by the installer,
//! or by the operating system. Its process id and its activation flag live in the data folder, so
//! a later launch finds the node again. The shell asks the node for `GET /v1/worker` on the loopback
//! control API, with the read token the node wrote. Errors are stable codes for the UI; no path or
//! token is ever returned.

use crate::control_client::{self, read_token};
use crate::journal::set_aside_stale_journal;
use crate::node_config::node_config;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// Loopback port of the node control API (fixed: the shell knows where the node is).
pub const CONTROL_PORT: u16 = 8750;
/// Network port of the node (the same as in `LISTEN`).
const NETWORK_PORT: u16 = 4001;
/// Render engine shipped beside the node (see `apps/desktop/scripts/prepare-sidecars.mjs`).
const RENDER_ENGINE: &str = "sil-raytracer-block";
/// Process id of the running node, written when it starts.
const PID_FILE: &str = "node.pid";
/// Present while the user wants the node to run (set by `start`, removed by `stop`).
const ACTIVE_FILE: &str = "active";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeError {
    NoBinary,
    PortBusy,
    StartFailed,
    NotRunning,
    Unreachable,
    Unauthorized,
    BadAnswer,
    Storage,
}

impl NodeError {
    /// Stable code sent to the UI.
    pub fn code(self) -> &'static str {
        match self {
            Self::NoBinary => "no_binary",
            Self::PortBusy => "port_busy",
            Self::StartFailed => "start_failed",
            Self::NotRunning => "not_running",
            Self::Unreachable => "unreachable",
            Self::Unauthorized => "unauthorized",
            Self::BadAnswer => "bad_answer",
            Self::Storage => "storage",
        }
    }
}

/// Owns the node's data folder and the node process started by this session, if any.
pub struct NodeProcess {
    data_dir: PathBuf,
    child: Mutex<Option<Child>>,
}

impl NodeProcess {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            child: Mutex::new(None),
        }
    }

    fn config_path(&self) -> PathBuf {
        self.data_dir.join("node.toml")
    }

    fn token_path(&self) -> PathBuf {
        self.data_dir.join("control.token")
    }

    fn pid_path(&self) -> PathBuf {
        self.data_dir.join(PID_FILE)
    }

    fn active_path(&self) -> PathBuf {
        self.data_dir.join(ACTIVE_FILE)
    }

    /// The user activated the node and has not deactivated it: it should run.
    pub fn wants_to_run(&self) -> bool {
        self.active_path().is_file()
    }

    /// The process id recorded by the last start, if any.
    fn recorded_pid(&self) -> Option<u32> {
        fs::read_to_string(self.pid_path()).ok()?.trim().parse().ok()
    }

    /// True while the node runs: the child this session started, or the recorded process.
    pub fn is_running(&self) -> bool {
        let owned = match self.child.lock() {
            Ok(mut guard) => guard.as_mut().map(|child| matches!(child.try_wait(), Ok(None))),
            Err(_) => None,
        };
        match owned {
            Some(alive) => alive,
            None => self.recorded_pid().is_some_and(pid_alive),
        }
    }

    /// Writes the node configuration and starts `silicium-node --roles worker,verifier`. A node that
    /// is already running is left alone. The node keeps running after this window closes.
    pub fn start(&self) -> Result<(), NodeError> {
        if self.is_running() {
            return Ok(());
        }
        // A busy port makes the node exit at once, while the window would go on talking to
        // whatever holds the port. Check before starting.
        if !ports_free() {
            return Err(NodeError::PortBusy);
        }
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf));
        let binary = resolve_binary(std::env::var("SIL_NODE_BIN").ok(), exe_dir.clone())
            .ok_or(NodeError::NoBinary)?;
        // The render engine ships next to the node: without it the node serves sha256 only.
        let engine = sibling(exe_dir.as_deref(), &exe_name(RENDER_ENGINE));

        fs::create_dir_all(&self.data_dir).map_err(|_| NodeError::Storage)?;
        set_aside_stale_journal(&self.data_dir).map_err(|_| NodeError::Storage)?;
        fs::write(
            self.config_path(),
            node_config(&self.data_dir, engine.as_deref()),
        )
        .map_err(|_| NodeError::Storage)?;
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.data_dir.join("node.log"))
            .map_err(|_| NodeError::Storage)?;
        let log_err = log.try_clone().map_err(|_| NodeError::Storage)?;

        let mut command = Command::new(binary);
        command
            .arg("--config")
            .arg(self.config_path())
            .arg("--roles")
            .arg("worker,verifier")
            .env("SIL_APP_VERSION", env!("CARGO_PKG_VERSION"))
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err));
        hide_window(&mut command);
        let child = command.spawn().map_err(|_| NodeError::StartFailed)?;
        fs::write(self.pid_path(), child.id().to_string()).map_err(|_| NodeError::Storage)?;
        fs::write(self.active_path(), b"1").map_err(|_| NodeError::Storage)?;
        let mut guard = self.child.lock().map_err(|_| NodeError::StartFailed)?;
        *guard = Some(child);
        Ok(())
    }

    /// Stops the node and clears the activation: the user deactivated this PC. Safe to call when
    /// the node is not running.
    pub fn stop(&self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            } else if let Some(pid) = self.recorded_pid() {
                kill_pid(pid);
            }
        }
        let _ = fs::remove_file(self.pid_path());
        let _ = fs::remove_file(self.active_path());
    }

    /// The node's own report (`GET /v1/worker`), as JSON.
    pub fn worker_report(&self) -> Result<Value, NodeError> {
        if !self.is_running() {
            return Err(NodeError::NotRunning);
        }
        let token = read_token(&self.token_path()).ok_or(NodeError::NotRunning)?;
        control_client::worker_report(&token)
    }
}

/// `SIL_NODE_BIN` if it points to a file, else `silicium-node` next to this program.
pub fn resolve_binary(env_path: Option<String>, exe_dir: Option<PathBuf>) -> Option<PathBuf> {
    let from_env = env_path.map(PathBuf::from).filter(|path| path.is_file());
    from_env.or_else(|| sibling(exe_dir.as_deref(), &exe_name("silicium-node")))
}

/// A program shipped next to this one, if it exists.
pub fn sibling(exe_dir: Option<&Path>, name: &str) -> Option<PathBuf> {
    exe_dir
        .map(|dir| dir.join(name))
        .filter(|path| path.is_file())
}

/// True when the node's control port and its network port can both be bound now.
fn ports_free() -> bool {
    port_free(("127.0.0.1", CONTROL_PORT)) && port_free(("0.0.0.0", NETWORK_PORT))
}

/// True when nothing listens on `address` (the port can be bound).
fn port_free(address: impl std::net::ToSocketAddrs) -> bool {
    std::net::TcpListener::bind(address).is_ok()
}

/// True when a process with this id exists.
fn pid_alive(pid: u32) -> bool {
    if cfg!(windows) {
        let mut command = Command::new("tasklist");
        command.args(["/FI", &format!("PID eq {pid}"), "/NH"]);
        hide_window(&mut command);
        command
            .output()
            .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()))
    } else {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .is_ok_and(|status| status.success())
    }
}

/// Ends the process with this id, if it exists.
fn kill_pid(pid: u32) {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/F"]);
        command
    } else {
        let mut command = Command::new("kill");
        command.args(["-9", &pid.to_string()]);
        command
    };
    hide_window(&mut command);
    let _ = command.status();
}

/// File name of a program on this platform.
pub fn exe_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_command: &mut Command) {}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "tests fail loudly on purpose")]
mod tests {
    use super::*;

    #[test]
    fn a_sibling_is_found_only_when_the_file_exists() {
        let dir = tempfile::tempdir().expect("dir");
        assert_eq!(sibling(Some(dir.path()), "missing"), None);
        fs::write(dir.path().join("present"), b"").expect("write");
        assert_eq!(
            sibling(Some(dir.path()), "present"),
            Some(dir.path().join("present"))
        );
        assert_eq!(sibling(None, "present"), None);
    }

    #[test]
    fn the_env_override_wins_when_the_file_exists() {
        let dir = tempfile::tempdir().expect("dir");
        let bin = dir.path().join("custom-node");
        fs::write(&bin, b"").expect("write");
        let found = resolve_binary(Some(bin.to_string_lossy().into_owned()), None);
        assert_eq!(found, Some(bin));
        assert_eq!(resolve_binary(Some("/does/not/exist".into()), None), None);
    }

    #[test]
    fn a_port_held_by_someone_else_is_busy() {
        let held = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let address = held.local_addr().expect("address");
        assert!(!port_free(address));
        drop(held);
        assert!(port_free(address));
    }

    #[test]
    fn a_stopped_node_has_no_report() {
        let dir = tempfile::tempdir().expect("dir");
        let process = NodeProcess::new(dir.path().to_path_buf());
        assert_eq!(process.worker_report(), Err(NodeError::NotRunning));
        assert_eq!(NodeError::Unreachable.code(), "unreachable");
    }

    #[test]
    fn activation_is_remembered_in_the_data_folder_until_stopped() {
        let dir = tempfile::tempdir().expect("dir");
        let process = NodeProcess::new(dir.path().to_path_buf());
        assert!(!process.wants_to_run());
        fs::write(dir.path().join(ACTIVE_FILE), b"1").expect("write");
        assert!(process.wants_to_run());
        // A recorded process that does not exist is not running, even when activated.
        fs::write(dir.path().join(PID_FILE), u32::MAX.to_string()).expect("write");
        assert!(!process.is_running());
        process.stop();
        assert!(!process.wants_to_run());
        assert!(!dir.path().join(PID_FILE).exists());
    }
}
