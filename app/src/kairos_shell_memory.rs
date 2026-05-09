//! Per-project shell memory for Kairos.
//!
//! When the user opens a new tab with an explicit shell (`AddTabWithShell`),
//! that choice is persisted to `.kairos/shell-pref.json` in the nearest
//! project root — the first ancestor directory containing `.git`, `.kairos`,
//! or `scaffold.yml`.
//!
//! On the next `AddDefaultTab` in the same project the saved shell preference
//! is loaded and used instead of the global startup-shell setting, giving each
//! project its own remembered shell without modifying global preferences.
//!
//! # File format
//! ```json
//! {
//!   "shell": { "WSL": "Ubuntu-24.04" }
//! }
//! ```
//! The `shell` field serialises a [`NewSessionShell`] value, which already
//! derives `Serialize`/`Deserialize`.
//!
//! # Scope disambiguation
//! `.kairos/shell-pref.json` lives at `<project-root>/.kairos/` (per-project
//! governance data).  This is distinct from the global Kairos app config dir
//! (renamed from `.openwarp`) which lives at a system-level path such as
//! `~/.kairos/` on Linux.  Both use the `.kairos` name but at different
//! filesystem levels; the project root walk anchors usage unambiguously.

use std::path::{Path, PathBuf};

use crate::terminal::session_settings::NewSessionShell;

// ---------------------------------------------------------------------------
// Project root detection
// ---------------------------------------------------------------------------

/// Walk up from `start` until a project-root marker is found (`.git`,
/// `.kairos`, or `scaffold.yml`).  Returns `start` itself when none is found.
///
/// The walk stops at filesystem roots (e.g. `C:\` on Windows, `/` on Unix)
/// to avoid traversing past the user's home directory.
pub fn find_project_root(start: &Path) -> PathBuf {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join(".git").exists()
            || dir.join(".kairos").exists()
            || dir.join("scaffold.yml").exists()
        {
            return dir;
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent.to_path_buf(),
            _ => return start.to_path_buf(),
        }
    }
}

// ---------------------------------------------------------------------------
// On-disk representation
// ---------------------------------------------------------------------------

/// JSON structure written to `.kairos/shell-pref.json`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ShellPref {
    /// Serialised [`NewSessionShell`] — covers all shell variants.
    shell: NewSessionShell,
}

fn pref_path(project_root: &Path) -> PathBuf {
    project_root.join(".kairos").join("shell-pref.json")
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Save `shell` as the preferred shell for the project that contains `cwd`.
///
/// Creates `.kairos/` inside the project root if it does not exist yet.
/// `SystemDefault` is not persisted — it is the implicit global default
/// and saving it would mask future changes to the startup-shell setting.
/// Silently ignores errors (logs a warning) so the caller flow is never
/// interrupted by filesystem issues.
pub fn save_shell_pref(cwd: &Path, shell: &NewSessionShell) {
    // Do not persist the system default — already the global implicit default.
    if matches!(shell, NewSessionShell::SystemDefault) {
        return;
    }

    let root = find_project_root(cwd);
    let kairos_dir = root.join(".kairos");

    if let Err(e) = std::fs::create_dir_all(&kairos_dir) {
        log::warn!(
            "kairos_shell_memory: could not create {}: {e}",
            kairos_dir.display()
        );
        return;
    }

    let pref = ShellPref {
        shell: shell.clone(),
    };
    let json = match serde_json::to_string_pretty(&pref) {
        Ok(j) => j,
        Err(e) => {
            log::warn!("kairos_shell_memory: serialise error: {e}");
            return;
        }
    };

    let path = pref_path(&root);
    if let Err(e) = std::fs::write(&path, &json) {
        log::warn!(
            "kairos_shell_memory: write failed for {}: {e}",
            path.display()
        );
    } else {
        log::debug!(
            "kairos_shell_memory: saved shell pref {:?} for {}",
            shell,
            root.display()
        );
    }
}

/// Load the shell preference for the project that contains `cwd`.
///
/// Returns `None` when no preference has been saved for this project, when the
/// saved file cannot be parsed, or when the walk reaches the filesystem root
/// without finding a project marker.
pub fn load_shell_pref(cwd: &Path) -> Option<NewSessionShell> {
    let root = find_project_root(cwd);
    let path = pref_path(&root);
    let bytes = std::fs::read(&path).ok()?;
    let pref: ShellPref = serde_json::from_slice(&bytes)
        .inspect_err(|e| {
            log::warn!(
                "kairos_shell_memory: failed to parse {}: {e}",
                path.display()
            )
        })
        .ok()?;
    log::debug!(
        "kairos_shell_memory: loaded shell pref {:?} for {}",
        pref.shell,
        root.display()
    );
    Some(pref.shell)
}
