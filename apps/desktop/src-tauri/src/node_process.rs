//! Runs the Silicium node as a worker, as a child process, and reads its worker report.
//!
//! The node is a separate program (`silicium-node`). This shell starts it, stops it and asks it
//! for `GET /v1/worker` on the loopback control API, with the read token the node wrote. Errors
//! are stable codes for the UI; no path or token is ever returned.

use crate::control_client::{self, read_token};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// Loopback port of the node control API (fixed: the shell knows where the node is).
pub const CONTROL_PORT: u16 = 8750;
const LISTEN: &str = "/ip4/0.0.0.0/tcp/4001";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeError {
    NoBinary,
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
            Self::StartFailed => "start_failed",
            Self::NotRunning => "not_running",
            Self::Unreachable => "unreachable",
            Self::Unauthorized => "unauthorized",
            Self::BadAnswer => "bad_answer",
            Self::Storage => "storage",
        }
    }
}

/// Owns the node's data folder and, when started, its child process.
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

    /// True while the child process has not exited.
    pub fn is_running(&self) -> bool {
        let Ok(mut guard) = self.child.lock() else {
            return false;
        };
        match guard.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }

    /// Writes the node configuration and starts `silicium-node --roles worker`. A node that is
    /// already running is left alone.
    pub fn start(&self) -> Result<(), NodeError> {
        if self.is_running() {
            return Ok(());
        }
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf));
        let binary = resolve_binary(std::env::var("SIL_NODE_BIN").ok(), exe_dir)
            .ok_or(NodeError::NoBinary)?;

        fs::create_dir_all(&self.data_dir).map_err(|_| NodeError::Storage)?;
        fs::write(self.config_path(), node_config(&self.data_dir))
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
            .arg("worker")
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err));
        hide_window(&mut command);
        let child = command.spawn().map_err(|_| NodeError::StartFailed)?;
        let mut guard = self.child.lock().map_err(|_| NodeError::StartFailed)?;
        *guard = Some(child);
        Ok(())
    }

    /// Stops the node. Safe to call when it is not running.
    pub fn stop(&self) {
        let Ok(mut guard) = self.child.lock() else {
            return;
        };
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
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

impl Drop for NodeProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

/// `SIL_NODE_BIN` if it points to a file, else `silicium-node` next to this program.
pub fn resolve_binary(env_path: Option<String>, exe_dir: Option<PathBuf>) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "silicium-node.exe"
    } else {
        "silicium-node"
    };
    let from_env = env_path.map(PathBuf::from).filter(|path| path.is_file());
    from_env.or_else(|| {
        exe_dir
            .map(|dir| dir.join(name))
            .filter(|path| path.is_file())
    })
}

/// The node configuration written before each start. Paths use forward slashes (valid on
/// Windows and parsed the same way by the node's TOML reader).
pub fn node_config(data_dir: &Path) -> String {
    let dir = data_dir.to_string_lossy().replace('\\', "/");
    format!(
        "data_dir = \"{dir}\"\n\n[network]\nlisten = [\"{LISTEN}\"]\nbootstrap = []\n\n[control]\nbind = \"127.0.0.1\"\nport = {CONTROL_PORT}\ntoken_file = \"{dir}/control.token\"\n"
    )
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
    fn config_points_the_node_at_its_folder_and_the_loopback_api() {
        let text = node_config(Path::new("C:\\Users\\x\\node"));
        assert!(text.contains("data_dir = \"C:/Users/x/node\""));
        assert!(text.contains("bind = \"127.0.0.1\""));
        assert!(text.contains("port = 8750"));
        assert!(text.contains("token_file = \"C:/Users/x/node/control.token\""));
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
    fn a_stopped_node_has_no_report() {
        let dir = tempfile::tempdir().expect("dir");
        let process = NodeProcess::new(dir.path().to_path_buf());
        assert_eq!(process.worker_report(), Err(NodeError::NotRunning));
        assert_eq!(NodeError::Unreachable.code(), "unreachable");
    }
}
