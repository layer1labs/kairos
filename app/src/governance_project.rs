//! Singleton model that tracks the active terminal's working directory for governance context.
//!
//! The workspace updates this model whenever the active pane group's working directory changes.
//! The Governance settings page subscribes to it to show per-project status and action buttons.

use std::path::PathBuf;
use warpui::{Entity, ModelContext, SingletonEntity};

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GovernanceProjectEvent {
    /// The active project directory changed.
    ActiveDirChanged,
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// Tracks which directory Kairos considers the "active governance project".
///
/// Updated by the workspace view whenever the active pane group's most-recent
/// working directory changes. The Governance settings page subscribes to this
/// model to show per-project status and action buttons.
pub struct GovernanceProjectState {
    /// Most recently active working directory from any terminal pane.
    pub active_dir: Option<PathBuf>,
    /// Whether a `.specsmith/` directory exists inside `active_dir`.
    pub has_specsmith: bool,
}

impl Default for GovernanceProjectState {
    fn default() -> Self {
        Self {
            active_dir: None,
            has_specsmith: false,
        }
    }
}

impl GovernanceProjectState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the active directory and recheck `.specsmith/` existence.
    pub fn set_active_dir(
        &mut self,
        dir: PathBuf,
        ctx: &mut ModelContext<Self>,
    ) {
        let new_has_specsmith = dir.join(".specsmith").is_dir();
        let changed = self.active_dir.as_ref() != Some(&dir)
            || self.has_specsmith != new_has_specsmith;
        if changed {
            self.active_dir = Some(dir);
            self.has_specsmith = new_has_specsmith;
            ctx.emit(GovernanceProjectEvent::ActiveDirChanged);
        }
    }
}

impl Entity for GovernanceProjectState {
    type Event = GovernanceProjectEvent;
}

/// Mark as application-wide singleton so it can be read from any view.
impl SingletonEntity for GovernanceProjectState {}
