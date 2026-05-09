//! Session configuration and specsmith command resolution.
//!
//! Handles platform-aware detection of the `specsmith` executable and
//! session-level configuration (project directory, governance port).
//!
//! # REQ-002 — Kairos Spawns specsmith serve as Managed Child Process
//! # REQ-008 — Local-Only Governance Communication

use std::path::PathBuf;
use std::time::Duration;

/// Runtime configuration for a Kairos session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Project root directory passed to `specsmith governance-serve` and
    /// included in preflight / verify requests so specsmith can resolve
    /// `docs/REQUIREMENTS.md` from the correct location.
    pub project_dir: PathBuf,

    /// The shell command used to invoke specsmith (resolved by
    /// [`find_specsmith_cmd`]).  Examples:
    /// - `"specsmith"` (globally installed via pipx)
    /// - `"py -m specsmith"` (Windows, specsmith installed in a user env)
    /// - `"python -m specsmith"` (Unix fallback)
    pub specsmith_cmd: String,

    /// Port on which `specsmith governance-serve` will be started.
    /// Defaults to 7700 (architecture constant, architecture invariant I2).
    pub governance_port: u16,

    /// Timeout for waiting for `specsmith governance-serve` to become healthy
    /// after spawning.
    pub startup_timeout: Duration,

    /// When `true`, do not spawn `specsmith governance-serve`; instead assume
    /// one is already running on `governance_port`.
    pub no_spawn: bool,
}

impl SessionConfig {
    /// Create a session configuration with sensible defaults.
    ///
    /// `project_dir` defaults to the current working directory if `None`.
    pub fn new(project_dir: Option<PathBuf>) -> Self {
        let project_dir = project_dir
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            project_dir,
            specsmith_cmd: find_specsmith_cmd(),
            governance_port: crate::governance::client::DEFAULT_PORT,
            startup_timeout: Duration::from_secs(15),
            no_spawn: false,
        }
    }

    /// Return the project directory as a UTF-8 string slice.
    pub fn project_dir_str(&self) -> &str {
        self.project_dir.to_str().unwrap_or(".")
    }
}

// ---------------------------------------------------------------------------
// Specsmith command resolution
// ---------------------------------------------------------------------------

/// Locate the `specsmith` executable, returning a command string suitable for
/// [`GovernanceServer::spawn`](crate::governance::server::GovernanceServer::spawn).
///
/// Resolution order:
/// 1. `SPECSMITH_CMD` environment variable (explicit override)
/// 2. `specsmith` binary in PATH (global install via pipx or system package)
/// 3. `py -m specsmith` (Windows, Python Launcher — covers pipx & venv installs)
/// 4. `python -m specsmith` (Unix / generic fallback)
///
/// # Example
/// ```
/// let cmd = kairos_governance::session::find_specsmith_cmd();
/// assert!(!cmd.is_empty());
/// ```
pub fn find_specsmith_cmd() -> String {
    // 1. Explicit env override — highest priority.
    if let Ok(cmd) = std::env::var("SPECSMITH_CMD") {
        if !cmd.trim().is_empty() {
            return cmd.trim().to_owned();
        }
    }

    // 2. Try `specsmith` directly (installed globally via pipx / system package).
    if probe_command("specsmith", &["--version"]) {
        return "specsmith".to_owned();
    }

    // 3. Windows: try `py -m specsmith` (Python Launcher ships with Python on Windows).
    #[cfg(target_os = "windows")]
    if probe_command("py", &["-m", "specsmith", "--version"]) {
        return "py -m specsmith".to_owned();
    }

    // 4. Unix / generic fallback.
    "python -m specsmith".to_owned()
}

/// Run `program args...` and return `true` if the process exits with code 0.
/// Errors (e.g. command not found) are silently treated as `false`.
fn probe_command(program: &str, args: &[&str]) -> bool {
    std::process::Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Log entry (shared between session and TUI)
// ---------------------------------------------------------------------------

/// A single timestamped log entry displayed in the TUI session log panel.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// ISO-8601 timestamp (local time, second precision).
    pub timestamp: String,
    /// Log level / category tag.
    pub level: LogLevel,
    /// Human-readable message.
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warn,
    Error,
}

impl LogEntry {
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, message)
    }
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Success, message)
    }
    pub fn warn(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warn, message)
    }
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, message)
    }

    fn new(level: LogLevel, message: impl Into<String>) -> Self {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        Self {
            timestamp,
            level,
            message: message.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_specsmith_cmd_returns_non_empty() {
        let cmd = find_specsmith_cmd();
        assert!(!cmd.is_empty(), "specsmith command must be non-empty");
    }

    #[test]
    fn session_config_defaults_valid() {
        let cfg = SessionConfig::new(None);
        assert!(cfg.governance_port > 0);
        assert!(!cfg.specsmith_cmd.is_empty());
        assert!(cfg.startup_timeout.as_secs() > 0);
        assert!(!cfg.no_spawn);
    }

    #[test]
    fn session_config_custom_project_dir() {
        let dir = std::path::PathBuf::from("/tmp/my-project");
        let cfg = SessionConfig::new(Some(dir.clone()));
        assert_eq!(cfg.project_dir, dir);
    }

    #[test]
    fn log_entry_levels_labelled() {
        let e = LogEntry::success("done");
        assert_eq!(e.level, LogLevel::Success);
        assert_eq!(e.message, "done");
        assert!(!e.timestamp.is_empty());
    }

    #[test]
    fn env_override_respected() {
        std::env::set_var("SPECSMITH_CMD", "my-specsmith");
        let cmd = find_specsmith_cmd();
        std::env::remove_var("SPECSMITH_CMD");
        assert_eq!(cmd, "my-specsmith");
    }
}
