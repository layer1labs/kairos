//! Context fill state — tracks the last known specsmith context window fill
//! percentage and the user-configured `num_ctx` override for Ollama models.
//!
//! This is a lightweight singleton model that:
//! - Is updated by `WorkspaceView` when it receives `context_fill` JSONL events
//!   from the specsmith governance stream (satisfying REQ-010 / REQ-021).
//! - Is read by `GovernancePageView` to render the always-visible fill bar.
//! - Persists `num_ctx` to `~/.specsmith/config.yml` via a specsmith subprocess.

use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

/// Current context-fill state, updated from specsmith JSONL events.
#[derive(Debug, Clone, Default)]
pub struct ContextFillState {
    /// Last known fill percentage [0.0, 1.0].  `None` if no event received yet.
    pub fill_pct: Option<f32>,
    /// User-configured `num_ctx` override (saved to ~/.specsmith/config.yml).
    /// `None` means the GPU-detection recommendation is used.
    pub custom_num_ctx: Option<u32>,
    /// Whether we have an unsaved pending `num_ctx` value being typed.
    pub pending_num_ctx_str: String,
    /// Whether the save operation is in progress.
    pub save_in_progress: bool,
    /// Last save result message ("" = none, "✓ saved", "✗ error: ...")
    pub save_result: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextFillEvent {
    FillUpdated,
    NumCtxSaved,
}

impl Entity for ContextFillState {
    type Event = ContextFillEvent;
}

impl SingletonEntity for ContextFillState {}

impl ContextFillState {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self::default()
    }

    /// Update the fill percentage.  Emits `FillUpdated`.
    pub fn set_fill(&mut self, pct: f32, ctx: &mut ModelContext<Self>) {
        let clamped = pct.clamp(0.0, 1.0);
        if self.fill_pct != Some(clamped) {
            self.fill_pct = Some(clamped);
            ctx.emit(ContextFillEvent::FillUpdated);
        }
    }

    /// Returns the fill % as a u8 (0–100), or None.
    pub fn fill_percent(&self) -> Option<u8> {
        self.fill_pct.map(|f| (f * 100.0).round() as u8)
    }

    /// Color tier for the fill bar.
    ///   green  < 60 %
    ///   yellow 60–79 %
    ///   red    ≥ 80 %
    pub fn fill_tier(&self) -> FillTier {
        match self.fill_pct {
            None => FillTier::Unknown,
            Some(f) if f < 0.60 => FillTier::Low,
            Some(f) if f < 0.80 => FillTier::Medium,
            _ => FillTier::High,
        }
    }

    /// Handle the GovernancePage editing `num_ctx`.
    pub fn set_pending_num_ctx(&mut self, s: &str) {
        self.pending_num_ctx_str = s.to_owned();
    }

    /// Save the pending `num_ctx` to specsmith config via subprocess.
    pub fn start_save(&mut self, ctx: &mut ModelContext<Self>) {
        let s = self.pending_num_ctx_str.trim().to_owned();
        // Validate: must parse as u32 in [512, 131072]
        let value: u32 = match s.parse::<u32>() {
            Ok(v) if (512..=131_072).contains(&v) => v,
            Ok(v) => {
                self.save_result = format!("✗ {} out of range [512, 131072]", v);
                return;
            }
            Err(_) => {
                self.save_result = "✗ invalid number".to_owned();
                return;
            }
        };
        self.save_in_progress = true;
        self.save_result.clear();
        ctx.spawn(
            async move {
                // Write num_ctx to ~/.specsmith/config.yml via specsmith config set
                let run = |prog: &str, args: &[String]| {
                    std::process::Command::new(prog)
                        .args(args)
                        .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                        .env("SPECSMITH_PYPI_CHECKED", "1")
                        .output()
                        .map_err(|e| e.to_string())
                };
                let args: Vec<String> = [
                    "config", "set", "ollama.num_ctx",
                    &value.to_string(),
                ]
                .iter()
                .map(|s| s.to_string())
                .collect();
                let py_args = {
                    let mut a = vec!["-m".to_string(), "specsmith".to_string()];
                    a.extend(args.clone());
                    a
                };
                run("py", &py_args)
                    .or_else(|_| run("specsmith", &args))
                    .map(|out| (value, out.status.success(), String::from_utf8_lossy(&out.stderr).to_string()))
                    .map_err(|e| e)
            },
            |me, result, ctx| {
                me.save_in_progress = false;
                match result {
                    Ok((v, true, _)) => {
                        me.custom_num_ctx = Some(v);
                        me.save_result = format!("\u{2713} Saved num_ctx = {}", v);
                        ctx.emit(ContextFillEvent::NumCtxSaved);
                    }
                    Ok((_, false, err)) => {
                        me.save_result = format!("\u{2717} {}", err.lines().next().unwrap_or("error"));
                    }
                    Err(e) => {
                        me.save_result = format!("\u{2717} specsmith not found: {e}");
                    }
                }
                ctx.notify();
            },
        );
    }

    /// Load the current num_ctx from specsmith config (best-effort, fire-and-forget).
    pub fn load_num_ctx(&mut self, ctx: &mut ModelContext<Self>) {
        ctx.spawn(
            async move {
                let run = |prog: &str, args: &[String]| {
                    std::process::Command::new(prog)
                        .args(args)
                        .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                        .env("SPECSMITH_PYPI_CHECKED", "1")
                        .output()
                        .map_err(|e| e.to_string())
                };
                let args: Vec<String> = ["config", "get", "ollama.num_ctx"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                let py_args = {
                    let mut a = vec!["-m".to_string(), "specsmith".to_string()];
                    a.extend(args.clone());
                    a
                };
                run("py", &py_args)
                    .or_else(|_| run("specsmith", &args))
                    .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
                    .ok()
            },
            |me, result, ctx| {
                if let Some(text) = result {
                    if let Ok(v) = text.trim().parse::<u32>() {
                        if (512..=131_072).contains(&v) {
                            me.custom_num_ctx = Some(v);
                            me.pending_num_ctx_str = v.to_string();
                            ctx.notify();
                        }
                    }
                }
            },
        );
    }
}

/// Colour tier for the context fill bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillTier {
    Unknown,
    Low,    // < 60 % — green
    Medium, // 60–79 % — yellow
    High,   // ≥ 80 % — red
}
