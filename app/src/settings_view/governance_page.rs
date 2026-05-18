//! Governance settings page — specsmith engine status, updater, project context, and links.
//!
//! Sections:
//!  1. Governance engine — live health indicator + BYOE endpoint
//!  2. Project context   — active project dir, .specsmith/ status, audit/init/sync buttons
//!  3. specsmith updater — installed version, Check for Updates / Update buttons (pipx)
//!  4. Links            — GitHub issue trackers for both repos

use super::{
    settings_page::{
        build_sub_header, render_separator, MatchData, PageType, SettingsPageEvent,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use crate::kairos_context_fill::ContextFillState;
use crate::view_components::{SubmittableTextInput, SubmittableTextInputEvent};
use kairos_governance::{AuditStatusResponse, GovernanceClient, GovernanceConfig};
use warpui::{
    elements::{
        ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element, Expanded,
        Flex, Hoverable, MouseStateHandle, ParentElement, Radius, Text,
    },
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
    },
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

// ---------------------------------------------------------------------------
// Governance health state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum HealthStatus {
    Unknown,
    Healthy { version: String },
    Unreachable { error: String },
}

// ---------------------------------------------------------------------------
// specsmith updater state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
enum UpdaterStatus {
    #[default]
    Idle,
    Checking,
    UpToDate {
        version: String,
    },
    UpdateAvailable {
        current: String,
        latest: String,
    },
    Updating,
    Updated {
        version: String,
    },
    Error {
        message: String,
    },
}

/// The persisted update channel (stable releases vs. pre-release dev builds).
#[derive(Debug, Clone, PartialEq, Default)]
enum UpdateChannel {
    /// Auto-detected from installed version — `.devN` suffix → dev, else stable.
    #[default]
    Unknown,
    Stable,
    Dev,
}

impl UpdateChannel {
    fn label(&self) -> &'static str {
        match self {
            Self::Unknown | Self::Stable => "stable",
            Self::Dev => "dev",
        }
    }
}

// ---------------------------------------------------------------------------
// Session context seed state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
enum ContextSeedStatus {
    #[default]
    Unknown,
    Loading,
    /// Loaded: (seed_turn_count, preview_of_first_turn)
    Loaded { turns: usize, preview: String },
    Cleared,
    Error(String),
}

// ---------------------------------------------------------------------------
// Dispatch runs state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
enum DispatchRunsStatus {
    #[default]
    Unknown,
    Loading,
    Loaded(Vec<String>),
    Error(String),
}

// ---------------------------------------------------------------------------
// Audit status state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
enum AuditStatusLocal {
    #[default]
    Unknown,
    Loading,
    Loaded(AuditStatusResponse),
    Error(String),
}

// ---------------------------------------------------------------------------
// CI/CD status state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
struct CiStatusData {
    ci_available: bool,
    last_run_status: String, // "success" | "failure" | "unknown"
    open_dep_alerts: i64,
    open_security_alerts: i64,
    error: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
enum CiStatus {
    #[default]
    Unknown,
    Loading,
    Loaded(CiStatusData),
    EnableRunning,
    EnableDone(String),
    EnableError(String),
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GovernancePageAction {
    CheckForSpecsmithUpdate,
    UpdateSpecsmith,
    /// Open a URL in the system browser (used by bug-report link rows).
    OpenLink(String),
    /// Refresh the current channel from `specsmith channel get --json`.
    RefreshChannel,
    /// Persist a channel preference via `specsmith channel set <channel>`.
    SetChannel(String),
    /// Refresh CI/CD status from `GET /api/ci/status`.
    RefreshCiStatus,
    /// Enable CI automation via `specsmith ci enable`.
    EnableCiAutomation,
    /// Trigger `specsmith context optimize`.
    OptimizeContext,
    /// Refresh the epistemic context seed from `GET /api/session/context-seed`.
    RefreshContextSeed,
    /// Clear session state via `POST /api/session/clear`.
    ClearContextSeed,
    /// Refresh the list of dispatch DAG runs.
    RefreshDispatchRuns,
    /// Refresh governance audit status from `GET /api/audit`.
    RefreshAuditStatus,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct GovernancePageView {
    page: PageType<Self>,
    health: HealthStatus,
    updater: UpdaterStatus,
    check_update_button: MouseStateHandle,
    do_update_button: MouseStateHandle,
    /// Current update channel (stable / dev), read from `specsmith channel get`.
    channel: UpdateChannel,
    channel_stable_button: MouseStateHandle,
    channel_dev_button: MouseStateHandle,
    /// Editable text input for `ollama.num_ctx` (REQ-022).
    pub(crate) num_ctx_input: ViewHandle<SubmittableTextInput>,
    /// CI/CD status from governance server.
    ci_status: CiStatus,
    ci_refresh_button: MouseStateHandle,
    ci_enable_button: MouseStateHandle,
    optimize_button: MouseStateHandle,
    /// Epistemic context seed (session continuity).
    context_seed: ContextSeedStatus,
    seed_refresh_button: MouseStateHandle,
    seed_clear_button: MouseStateHandle,
    /// Multi-agent dispatch run list.
    dispatch_runs: DispatchRunsStatus,
    dispatch_refresh_button: MouseStateHandle,
    /// Governance audit health status.
    audit_status: AuditStatusLocal,
    audit_refresh_button: MouseStateHandle,
}

impl GovernancePageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        // Subscribe to ContextFillState so the page re-renders on fill updates.
        let fill_handle = ContextFillState::handle(ctx);
        ctx.subscribe_to_model(
            &fill_handle,
            |_me, _handle, _event: &crate::kairos_context_fill::ContextFillEvent, ctx| {
                ctx.notify();
            },
        );
        // Load the persisted num_ctx value from specsmith config.
        fill_handle.update(ctx, |state, model_ctx| {
            state.load_num_ctx(model_ctx);
        });

        let num_ctx_input = ctx.add_typed_action_view(|ctx| {
            let mut input = SubmittableTextInput::new(ctx);
            input.set_placeholder_text("num_ctx (e.g. 8192)".to_owned(), ctx);
            input
        });
        ctx.subscribe_to_view(&num_ctx_input, Self::on_num_ctx_event);

        let mut view = GovernancePageView {
            page: PageType::new_monolith(GovernancePageWidget::default(), None, false),
            health: HealthStatus::Unknown,
            updater: UpdaterStatus::Idle,
            check_update_button: MouseStateHandle::default(),
            do_update_button: MouseStateHandle::default(),
            channel: UpdateChannel::Unknown,
            channel_stable_button: MouseStateHandle::default(),
            channel_dev_button: MouseStateHandle::default(),
            num_ctx_input,
            ci_status: CiStatus::Unknown,
            ci_refresh_button: MouseStateHandle::default(),
            ci_enable_button: MouseStateHandle::default(),
            optimize_button: MouseStateHandle::default(),
            context_seed: ContextSeedStatus::Unknown,
            seed_refresh_button: MouseStateHandle::default(),
            seed_clear_button: MouseStateHandle::default(),
            dispatch_runs: DispatchRunsStatus::Unknown,
            dispatch_refresh_button: MouseStateHandle::default(),
            audit_status: AuditStatusLocal::Unknown,
            audit_refresh_button: MouseStateHandle::default(),
        };
        view.check_health(ctx);
        view.refresh_channel(ctx);
        view.fetch_ci_status(ctx);
        view.fetch_context_seed(ctx);
        view.fetch_dispatch_runs(ctx);
        view.fetch_audit_status(ctx);
        view
    }

    fn on_num_ctx_event(
        &mut self,
        _handle: ViewHandle<SubmittableTextInput>,
        event: &SubmittableTextInputEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        if let SubmittableTextInputEvent::Submit(text) = event {
            let text = text.clone();
            ContextFillState::handle(ctx).update(ctx, move |state, model_ctx| {
                state.set_pending_num_ctx(&text);
                state.start_save(model_ctx);
            });
            ctx.notify();
        }
    }

    /// Reads the current channel from `specsmith channel get --json`.
    fn refresh_channel(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.spawn(
            async move {
                // Try `py -m specsmith channel get --json` then `specsmith channel get --json`.
                let run = |prog: &str, args: &[&str]| -> Result<std::process::Output, String> {
                    std::process::Command::new(prog)
                        .args(args)
                        .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                        .env("SPECSMITH_PYPI_CHECKED", "1")
                        .output()
                        .map_err(|e| e.to_string())
                };
                let out = run("py", &["-m", "specsmith", "channel", "get", "--json"])
                    .or_else(|_| run("specsmith", &["channel", "get", "--json"]))
                    .map_err(|e| format!("specsmith not found: {e}"))?;
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                Ok(text)
            },
            |me, result: Result<String, String>, ctx| {
                if let Ok(text) = result {
                    // parse {"channel": "stable", "source": "..."}  (best-effort)
                    if text.contains("\"dev\"") {
                        me.channel = UpdateChannel::Dev;
                    } else if text.contains("\"stable\"") {
                        me.channel = UpdateChannel::Stable;
                    }
                    ctx.notify();
                }
            },
        );
    }

    /// Persists a channel preference via `specsmith channel set <channel>`.
    fn set_channel(&mut self, channel: &str, ctx: &mut ViewContext<Self>) {
        let channel = channel.to_owned();
        let channel_clone = channel.clone();
        ctx.spawn(
            async move {
                let run = |prog: &str, args: &[&str]| -> Result<std::process::Output, String> {
                    std::process::Command::new(prog)
                        .args(args)
                        .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                        .env("SPECSMITH_PYPI_CHECKED", "1")
                        .output()
                        .map_err(|e| e.to_string())
                };
                run("py", &["-m", "specsmith", "channel", "set", &channel])
                    .or_else(|_| run("specsmith", &["channel", "set", &channel]))
                    .map_err(|e| format!("specsmith not found: {e}"))?;
                Ok(())
            },
            move |me, result: Result<(), String>, ctx| {
                if result.is_ok() {
                    me.channel = if channel_clone == "dev" {
                        UpdateChannel::Dev
                    } else {
                        UpdateChannel::Stable
                    };
                    ctx.notify();
                }
            },
        );
    }

    fn fetch_ci_status(&mut self, ctx: &mut ViewContext<Self>) {
        self.ci_status = CiStatus::Loading;
        ctx.notify();
        ctx.spawn(
            async move {
                // Call GET http://127.0.0.1:7700/api/ci/status
                let url = "http://127.0.0.1:7700/api/ci/status";
                let out = std::process::Command::new("curl")
                    .args(["-s", "--max-time", "5", url])
                    .output()
                    .map_err(|e| e.to_string())?;
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                Ok(text)
            },
            |me, result: Result<String, String>, ctx| {
                me.ci_status = match result {
                    Ok(text) => {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                            CiStatus::Loaded(CiStatusData {
                                ci_available: v
                                    .get("ci_available")
                                    .and_then(|x| x.as_bool())
                                    .unwrap_or(false),
                                last_run_status: v
                                    .get("last_run_status")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("unknown")
                                    .to_owned(),
                                open_dep_alerts: v
                                    .get("open_dep_alerts")
                                    .and_then(|x| x.as_i64())
                                    .unwrap_or(0),
                                open_security_alerts: v
                                    .get("open_security_alerts")
                                    .and_then(|x| x.as_i64())
                                    .unwrap_or(0),
                                error: v
                                    .get("error")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_owned(),
                            })
                        } else {
                            CiStatus::Unknown
                        }
                    }
                    Err(_) => CiStatus::Unknown,
                };
                ctx.notify();
            },
        );
    }

    fn run_ci_enable(&mut self, ctx: &mut ViewContext<Self>) {
        self.ci_status = CiStatus::EnableRunning;
        ctx.notify();
        ctx.spawn(
            async move {
                let run = |prog: &str, args: &[&str]| -> Result<std::process::Output, String> {
                    std::process::Command::new(prog)
                        .args(args)
                        .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                        .env("SPECSMITH_PYPI_CHECKED", "1")
                        .output()
                        .map_err(|e| e.to_string())
                };
                let args = &["ci", "enable"];
                run("py", &{
                    let mut v = vec!["-m", "specsmith"];
                    v.extend_from_slice(args);
                    v
                })
                .or_else(|_| run("specsmith", args))
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .map_err(|e| e)
            },
            |me, result: Result<String, String>, ctx| {
                me.ci_status = match result {
                    Ok(out) => {
                        CiStatus::EnableDone(out.lines().take(5).collect::<Vec<_>>().join(" │ "))
                    }
                    Err(e) => CiStatus::EnableError(e.chars().take(120).collect()),
                };
                ctx.notify();
            },
        );
    }

    fn run_context_optimize(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.spawn(
            async move {
                let run = |prog: &str, args: &[&str]| -> Result<std::process::Output, String> {
                    std::process::Command::new(prog)
                        .args(args)
                        .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                        .env("SPECSMITH_PYPI_CHECKED", "1")
                        .output()
                        .map_err(|e| e.to_string())
                };
                let args = &["context", "optimize"];
                run("py", &{
                    let mut v = vec!["-m", "specsmith"];
                    v.extend_from_slice(args);
                    v
                })
                .or_else(|_| run("specsmith", args))
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .map_err(|e| e)
            },
            |_me, _result: Result<String, String>, ctx| {
                ctx.notify();
            },
        );
    }

    fn fetch_context_seed(&mut self, ctx: &mut ViewContext<Self>) {
        self.context_seed = ContextSeedStatus::Loading;
        ctx.notify();
        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                client.context_seed().await
            },
            |me, result, ctx| {
                me.context_seed = match result {
                    Ok(resp) if resp.ok => {
                        let preview = resp
                            .seed
                            .first()
                            .map(|t| {
                                let s = t.content.chars().take(80).collect::<String>();
                                if t.content.len() > 80 {
                                    format!("{}\u{2026}", s)
                                } else {
                                    s
                                }
                            })
                            .unwrap_or_default();
                        ContextSeedStatus::Loaded {
                            turns: resp.seed_turns,
                            preview,
                        }
                    }
                    Ok(resp) => ContextSeedStatus::Error(resp.project_dir),
                    Err(e) => ContextSeedStatus::Error(format!("{e:#}")),
                };
                ctx.notify();
            },
        );
    }

    fn clear_context_seed(&mut self, ctx: &mut ViewContext<Self>) {
        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                client.session_clear().await
            },
            |me, result, ctx| {
                me.context_seed = match result {
                    Ok(resp) if resp.ok => ContextSeedStatus::Cleared,
                    Ok(resp) => ContextSeedStatus::Error(resp.error),
                    Err(e) => ContextSeedStatus::Error(format!("{e:#}")),
                };
                ctx.notify();
            },
        );
    }

    fn fetch_dispatch_runs(&mut self, ctx: &mut ViewContext<Self>) {
        self.dispatch_runs = DispatchRunsStatus::Loading;
        ctx.notify();
        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                client.dispatch_list().await
            },
            |me, result, ctx| {
                me.dispatch_runs = match result {
                    Ok(resp) => DispatchRunsStatus::Loaded(resp.runs),
                    Err(e) => DispatchRunsStatus::Error(format!("{e:#}")),
                };
                ctx.notify();
            },
        );
    }

    fn fetch_audit_status(&mut self, ctx: &mut ViewContext<Self>) {
        self.audit_status = AuditStatusLocal::Loading;
        ctx.notify();
        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                client.audit_status().await
            },
            |me, result, ctx| {
                me.audit_status = match result {
                    Ok(resp) => AuditStatusLocal::Loaded(resp),
                    Err(e) => AuditStatusLocal::Error(format!("{e:#}")),
                };
                ctx.notify();
            },
        );
    }

    fn check_health(&mut self, ctx: &mut ViewContext<Self>) {
        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                client.health().await
            },
            |me, result, ctx| {
                me.health = match result {
                    Ok(resp) => HealthStatus::Healthy {
                        version: resp.version,
                    },
                    Err(e) => HealthStatus::Unreachable {
                        error: format!("{e:#}"),
                    },
                };
                ctx.notify();
            },
        );
    }

    /// Runs `pipx upgrade specsmith`
    /// updates `updater` state.  Tries package managers in order:
    ///   1. pipx upgrade specsmith  (preferred)
    ///   2. pip install --upgrade specsmith
    ///   3. pip3 install --upgrade specsmith
    fn run_pipx_upgrade(&mut self, ctx: &mut ViewContext<Self>) {
        self.updater = UpdaterStatus::Updating;
        ctx.notify();
        ctx.spawn(
            async move {
                fn try_upgrade(prog: &str, args: &[&str]) -> Result<std::process::Output, String> {
                    std::process::Command::new(prog)
                        .args(args)
                        .output()
                        .map_err(|e| e.to_string())
                }
                let out = try_upgrade("pipx", &["upgrade", "specsmith"])
                    .or_else(|_| try_upgrade("pip", &["install", "--upgrade", "specsmith"]))
                    .or_else(|_| try_upgrade("pip3", &["install", "--upgrade", "specsmith"]))
                    .map_err(|e| format!("No package manager found (pipx/pip/pip3): {e}"))?;
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                if out.status.success() {
                    Ok(format!("{stdout}{stderr}"))
                } else {
                    Err(format!("{stdout}{stderr}"))
                }
            },
            |me, result: Result<String, String>, ctx| {
                me.updater = match result {
                    Ok(output) => {
                        let lower = output.to_lowercase();
                        if lower.contains("already at latest") {
                            let ver = extract_version_from_output(&output)
                                .unwrap_or_else(|| "latest".to_owned());
                            UpdaterStatus::UpToDate { version: ver }
                        } else if lower.contains("upgraded") {
                            let ver = extract_version_from_output(&output)
                                .unwrap_or_else(|| "latest".to_owned());
                            UpdaterStatus::Updated { version: ver }
                        } else {
                            UpdaterStatus::UpToDate {
                                version: "latest".to_owned(),
                            }
                        }
                    }
                    Err(e) => UpdaterStatus::Error {
                        message: e.chars().take(120).collect::<String>(),
                    },
                };
                ctx.notify();
            },
        );
    }

    /// Check the installed specsmith version.  Tries package managers in order:
    ///   1. pipx list --short     → parses the specsmith line
    ///   2. pip show specsmith    → parses the Version: field
    ///   3. pip3 show specsmith
    fn check_for_update(&mut self, ctx: &mut ViewContext<Self>) {
        self.updater = UpdaterStatus::Checking;
        ctx.notify();
        ctx.spawn(
            async move {
                // 1. pipx list --short
                if let Ok(out) = std::process::Command::new("pipx")
                    .args(["list", "--short"])
                    .output()
                {
                    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
                    if text.contains("specsmith") {
                        return Ok((
                            "pipx".to_owned(),
                            String::from_utf8_lossy(&out.stdout).to_string(),
                        ));
                    }
                }
                // 2/3. pip / pip3 show specsmith
                for prog in &["pip", "pip3"] {
                    if let Ok(out) = std::process::Command::new(prog)
                        .args(["show", "specsmith"])
                        .output()
                    {
                        if out.status.success() {
                            return Ok((
                                prog.to_string(),
                                String::from_utf8_lossy(&out.stdout).to_string(),
                            ));
                        }
                    }
                }
                Err("specsmith not found via pipx, pip, or pip3".to_owned())
            },
            |me, result: Result<(String, String), String>, ctx| {
                me.updater = match result {
                    Ok((manager, output)) => {
                        // pipx format: "  specsmith 0.10.1"
                        // pip format:  "Version: 0.10.1"
                        let current = if manager == "pipx" {
                            output
                                .lines()
                                .find(|l| l.to_lowercase().contains("specsmith"))
                                .and_then(|l| l.split_whitespace().nth(1))
                                .map(|v| v.to_owned())
                                .unwrap_or_else(|| "unknown".to_owned())
                        } else {
                            output
                                .lines()
                                .find(|l| l.to_lowercase().starts_with("version:"))
                                .and_then(|l| l.splitn(2, ':').nth(1))
                                .map(|v| v.trim().to_owned())
                                .unwrap_or_else(|| "unknown".to_owned())
                        };
                        UpdaterStatus::UpToDate { version: current }
                    }
                    Err(e) => UpdaterStatus::Error {
                        message: e.chars().take(120).collect::<String>(),
                    },
                };
                ctx.notify();
            },
        );
    }
}

/// Extracts a semver-like version string from pipx upgrade output.
fn extract_version_from_output(output: &str) -> Option<String> {
    for word in output.split_whitespace() {
        if word.starts_with(|c: char| c.is_ascii_digit()) && word.contains('.') {
            return Some(word.to_owned());
        }
    }
    None
}

impl Entity for GovernancePageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for GovernancePageView {
    type Action = GovernancePageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            GovernancePageAction::CheckForSpecsmithUpdate => {
                self.check_for_update(ctx);
            }
            GovernancePageAction::UpdateSpecsmith => {
                self.run_pipx_upgrade(ctx);
            }
            GovernancePageAction::RefreshChannel => {
                self.refresh_channel(ctx);
            }
            GovernancePageAction::SetChannel(channel) => {
                self.set_channel(channel, ctx);
            }
            GovernancePageAction::OpenLink(url) => {
                ctx.open_url(url);
            }
            GovernancePageAction::RefreshCiStatus => {
                self.fetch_ci_status(ctx);
            }
            GovernancePageAction::EnableCiAutomation => {
                self.run_ci_enable(ctx);
            }
            GovernancePageAction::OptimizeContext => {
                self.run_context_optimize(ctx);
            }
            GovernancePageAction::RefreshContextSeed => {
                self.fetch_context_seed(ctx);
            }
            GovernancePageAction::ClearContextSeed => {
                self.clear_context_seed(ctx);
            }
            GovernancePageAction::RefreshDispatchRuns => {
                self.fetch_dispatch_runs(ctx);
            }
            GovernancePageAction::RefreshAuditStatus => {
                self.fetch_audit_status(ctx);
            }
        }
    }
}

impl View for GovernancePageView {
    fn ui_name() -> &'static str {
        "GovernancePage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

#[derive(Default)]
struct GovernancePageWidget {}

impl GovernancePageWidget {
    fn action_button(
        label: impl Into<String>,
        mouse_state: MouseStateHandle,
        action: GovernancePageAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, mouse_state)
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label(label.into())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(action.clone());
            })
            .finish()
    }

    fn card(content: Box<dyn Element>, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(content)
            .with_background(appearance.theme().surface_1())
            .with_uniform_padding(16.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_margin_bottom(12.)
            .finish()
    }

    fn dim_label(text: impl Into<String>, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(
            Text::new(text.into(), appearance.monospace_font_family(), 11.)
                .with_color(appearance.theme().disabled_ui_text_color().into())
                .finish(),
        )
        .with_margin_top(4.)
        .finish()
    }
}

impl SettingsWidget for GovernancePageWidget {
    type View = GovernancePageView;

    fn search_terms(&self) -> &str {
        "governance specsmith local ai engine BYOE endpoint port 7700 update pipx channel stable dev bugs report session context seed clear dispatch runs audit health compliance esdb"
    }

    fn render(
        &self,
        view: &GovernancePageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color();

        // ── Section 1: Governance engine ─────────────────────────────────
        let engine_header = build_sub_header(appearance, "Local AI Governance", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let (dot_color, status_text) = match &view.health {
            HealthStatus::Unknown => (
                dim,
                "governance-serve \u{2014} checking\u{2026}".to_string(),
            ),
            HealthStatus::Healthy { version } => (
                theme.accent().into_solid().into(),
                format!("governance-serve  online  v{version}"),
            ),
            HealthStatus::Unreachable { .. } => (
                dim,
                "governance-serve  offline  (specsmith not running)".to_string(),
            ),
        };

        let dot = Text::new_inline("\u{25CF}", appearance.ui_font_family(), 13.)
            .with_color(dot_color.into())
            .finish();

        let status_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Container::new(dot).with_margin_right(8.).finish())
            .with_child(
                Text::new_inline(status_text, appearance.ui_font_family(), 13.)
                    .with_color(active.into())
                    .finish(),
            )
            .finish();

        let endpoint_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(
                    Text::new_inline("BYOE endpoint", appearance.ui_font_family(), 12.)
                        .with_color(dim.into())
                        .finish(),
                )
                .with_margin_right(8.)
                .finish(),
            )
            .with_child(Self::dim_label("http://127.0.0.1:7700/v1/", appearance))
            .finish();

        let desc_text = "Kairos spawns specsmith as a managed child process at startup. \
            All AI governance \u{2014} preflight checks, verification, confidence scoring, \
            and audit \u{2014} runs locally on your machine with no external network calls.";

        let desc = Container::new(
            Text::new(desc_text.to_string(), appearance.ui_font_family(), 12.)
                .with_color(dim.into())
                .soft_wrap(true)
                .finish(),
        )
        .with_margin_top(10.)
        .finish();

        let engine_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(status_row)
                .with_child(Container::new(endpoint_row).with_margin_top(8.).finish())
                .with_child(desc)
                .finish(),
            appearance,
        );

        // ── Section 2: specsmith updater ──────────────────────────────────
        // (Active Project controls are in the Governance tools panel in the left sidebar.)
        let updater_header = build_sub_header(appearance, "specsmith Updates", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let (updater_status_text, updater_color) = match &view.updater {
            UpdaterStatus::Idle => (
                "Click \"Check for updates\" to check the installed version.".to_string(),
                dim,
            ),
            UpdaterStatus::Checking => ("Checking\u{2026}".to_string(), dim),
            UpdaterStatus::UpToDate { version } => {
                (format!("specsmith {version}  \u{2014}  up to date"), active)
            }
            UpdaterStatus::UpdateAvailable { current, latest } => (
                format!("Update available: {current} \u{2192} {latest}"),
                active,
            ),
            UpdaterStatus::Updating => ("Updating specsmith\u{2026}".to_string(), dim),
            UpdaterStatus::Updated { version } => (
                format!("Updated to specsmith {version}"),
                theme.accent().into_solid().into(),
            ),
            UpdaterStatus::Error { message } => (format!("Error: {message}"), dim),
        };

        // ── Channel selector ─────────────────────────────────────────────
        let is_dev = matches!(view.channel, UpdateChannel::Dev);
        let is_stable = matches!(view.channel, UpdateChannel::Stable | UpdateChannel::Unknown);
        let channel_label_color = dim;

        let stable_btn = {
            let variant = if is_stable {
                ButtonVariant::Accent
            } else {
                ButtonVariant::Secondary
            };
            appearance
                .ui_builder()
                .button(variant, view.channel_stable_button.clone())
                .with_style(UiComponentStyles {
                    font_size: Some(12.),
                    padding: Some(Coords::uniform(6.)),
                    ..Default::default()
                })
                .with_centered_text_label("stable".to_string())
                .build()
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(GovernancePageAction::SetChannel(
                        "stable".to_owned(),
                    ));
                })
                .finish()
        };

        let dev_btn = {
            let variant = if is_dev {
                ButtonVariant::Accent
            } else {
                ButtonVariant::Secondary
            };
            appearance
                .ui_builder()
                .button(variant, view.channel_dev_button.clone())
                .with_style(UiComponentStyles {
                    font_size: Some(12.),
                    padding: Some(Coords::uniform(6.)),
                    ..Default::default()
                })
                .with_centered_text_label("dev".to_string())
                .build()
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(GovernancePageAction::SetChannel("dev".to_owned()));
                })
                .finish()
        };

        let channel_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(
                    Text::new_inline("Update channel", appearance.ui_font_family(), 12.)
                        .with_color(channel_label_color.into())
                        .finish(),
                )
                .with_margin_right(12.)
                .finish(),
            )
            .with_child(Container::new(stable_btn).with_margin_right(6.).finish())
            .with_child(dev_btn)
            .finish();

        let channel_hint = Text::new(
            if is_dev {
                "dev  \u{2014}  receives pre-release builds (.devN)".to_string()
            } else {
                "stable  \u{2014}  production releases only".to_string()
            },
            appearance.ui_font_family(),
            11.,
        )
        .with_color(dim.into())
        .finish();

        let updater_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(
                    Text::new(updater_status_text, appearance.ui_font_family(), 12.)
                        .with_color(updater_color.into())
                        .soft_wrap(true)
                        .finish(),
                )
                .with_child(
                    Container::new(
                        Flex::row()
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_child(
                                Container::new(Self::action_button(
                                    "Check for updates",
                                    view.check_update_button.clone(),
                                    GovernancePageAction::CheckForSpecsmithUpdate,
                                    appearance,
                                ))
                                .with_margin_right(8.)
                                .finish(),
                            )
                            .with_child(Self::action_button(
                                "Update specsmith",
                                view.do_update_button.clone(),
                                GovernancePageAction::UpdateSpecsmith,
                                appearance,
                            ))
                            .finish(),
                    )
                    .with_margin_top(10.)
                    .finish(),
                )
                .with_child(Container::new(channel_row).with_margin_top(12.).finish())
                .with_child(Container::new(channel_hint).with_margin_top(4.).finish())
                .with_child(
                    Container::new(
                        Text::new(
                            "Install: pip install specsmith   or   pipx install specsmith"
                                .to_string(),
                            appearance.monospace_font_family(),
                            11.,
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .with_margin_top(8.)
                    .finish(),
                )
                .finish(),
            appearance,
        );

        // ── Section 3: Bug report links — accent button rows ─────────────
        let links_header = build_sub_header(appearance, "Report Bugs", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let make_link_btn = |label: &str,
                             url: &str,
                             mouse_state: warpui::elements::MouseStateHandle,
                             appearance: &Appearance|
         -> Box<dyn Element> {
            let full_url = format!("https://{url}");
            let url_display = url.to_owned();
            let label_str = label.to_string();
            let font_family = appearance.ui_font_family();
            let mono_family = appearance.monospace_font_family();
            let dim_c = dim;
            let accent_c = theme.accent().into_solid();
            Hoverable::new(mouse_state, move |_hovered| {
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(
                        Expanded::new(
                            1.,
                            Text::new_inline(label_str.clone(), font_family.clone(), 12.)
                                .with_color(dim_c.into())
                                .finish(),
                        )
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new_inline(
                                format!("\u{2192}  {url_display}"),
                                mono_family.clone(),
                                11.,
                            )
                            .with_color(accent_c.into())
                            .finish(),
                        )
                        .with_margin_left(12.)
                        .finish(),
                    )
                    .finish()
            })
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(GovernancePageAction::OpenLink(full_url.clone()));
            })
            .with_cursor(warpui::platform::Cursor::PointingHand)
            .finish()
        };

        // Allocate separate MouseStateHandles for each link button.
        let specsmith_link_ms = warpui::elements::MouseStateHandle::default();
        let kairos_link_ms = warpui::elements::MouseStateHandle::default();

        let links_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(make_link_btn(
                    "Governance / specsmith bugs",
                    "github.com/BitConcepts/specsmith/issues",
                    specsmith_link_ms,
                    appearance,
                ))
                .with_child(
                    Container::new(make_link_btn(
                        "Kairos terminal bugs",
                        "github.com/BitConcepts/kairos/issues",
                        kairos_link_ms,
                        appearance,
                    ))
                    .with_margin_top(8.)
                    .finish(),
                )
                .finish(),
            appearance,
        );

        // ── Section: Context Window (REQ-021 / REQ-022) ─────────────────
        let fill_state = ContextFillState::as_ref(_app);
        let fill_pct = fill_state.fill_pct;
        let save_result = fill_state.save_result.clone();
        let custom_num_ctx = fill_state.custom_num_ctx;

        let (fill_dot_color, fill_text) = match fill_pct {
            None => (
                dim,
                "No context fill data yet — will update on first AI interaction.".to_string(),
            ),
            Some(f) => {
                use crate::kairos_context_fill::FillTier;
                let color = match fill_state.fill_tier() {
                    FillTier::Low => theme.accent().into_solid().into(),
                    FillTier::Medium => active,
                    FillTier::High | FillTier::Unknown => dim,
                };
                (color, format!("{:.0}% context window used", f * 100.0))
            }
        };

        let fill_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(
                    Text::new_inline("\u{25CF}", appearance.ui_font_family(), 13.)
                        .with_color(fill_dot_color.into())
                        .finish(),
                )
                .with_margin_right(8.)
                .finish(),
            )
            .with_child(
                Text::new_inline(fill_text, appearance.ui_font_family(), 12.)
                    .with_color(active.into())
                    .finish(),
            )
            .finish();

        let num_ctx_label = match custom_num_ctx {
            None => "Context size (Ollama num_ctx)  \u{2014}  auto".to_string(),
            Some(v) => format!("Context size (Ollama num_ctx)  \u{2014}  {v} tokens"),
        };

        let ctx_win_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(fill_row)
                .with_child(
                    Container::new(
                        Text::new_inline(num_ctx_label, appearance.ui_font_family(), 12.)
                            .with_color(dim.into())
                            .finish(),
                    )
                    .with_margin_top(10.)
                    .finish(),
                )
                .with_child(
                    Container::new(ChildView::new(&view.num_ctx_input).finish())
                        .with_margin_top(6.)
                        .finish(),
                )
                .with_child(
                    Container::new(
                        Text::new_inline(save_result, appearance.ui_font_family(), 11.)
                            .with_color(dim.into())
                            .finish(),
                    )
                    .with_margin_top(4.)
                    .finish(),
                )
                .with_child(Self::dim_label(
                    "Enter a value (512 \u{2013} 131072) and press Enter to save.",
                    appearance,
                ))
                .finish(),
            appearance,
        );

        let ctx_win_header = build_sub_header(appearance, "Context Window", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        // ── Section: Context Window — add Optimize button ────────────────
        // Wrap the existing fill row with an Optimize button inline.
        let optimize_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.optimize_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(11.),
                padding: Some(Coords::uniform(4.)),
                ..Default::default()
            })
            .with_centered_text_label("Optimize".to_string())
            .build()
            .on_click(|ctx, _, _| ctx.dispatch_typed_action(GovernancePageAction::OptimizeContext))
            .finish();

        // ── Section: CI / CD ───────────────────────────────────────────────
        let (ci_dot_color, ci_status_text) = match &view.ci_status {
            CiStatus::Unknown | CiStatus::Loading => (dim, "Checking CI status…".to_string()),
            CiStatus::EnableRunning => (dim, "Enabling CI automation…".to_string()),
            CiStatus::EnableDone(msg) => (active, format!("CI enabled: {msg}")),
            CiStatus::EnableError(e) => (theme.ui_error_color().into(), format!("Error: {e}")),
            CiStatus::Loaded(data) => {
                let color = if !data.ci_available {
                    dim
                } else if data.last_run_status == "success" {
                    theme.accent().into_solid().into()
                } else if data.last_run_status == "failure" {
                    theme.ui_error_color().into()
                } else {
                    dim
                };
                let mut parts = vec![format!("CI: {}", data.last_run_status)];
                if data.open_dep_alerts > 0 {
                    parts.push(format!("\u{26a0} {} dep alert(s)", data.open_dep_alerts));
                }
                if data.open_security_alerts > 0 {
                    parts.push(format!(
                        "\u{26a0} {} security alert(s)",
                        data.open_security_alerts
                    ));
                }
                if !data.error.is_empty() {
                    parts.push(format!("({})", &data.error[..data.error.len().min(60)]));
                }
                (color, parts.join("  "))
            }
        };

        let ci_refresh_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.ci_refresh_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(11.),
                padding: Some(Coords::uniform(4.)),
                ..Default::default()
            })
            .with_centered_text_label("Refresh".to_string())
            .build()
            .on_click(|ctx, _, _| ctx.dispatch_typed_action(GovernancePageAction::RefreshCiStatus))
            .finish();

        let ci_enable_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.ci_enable_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(11.),
                padding: Some(Coords::uniform(4.)),
                ..Default::default()
            })
            .with_centered_text_label("Enable CI".to_string())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(GovernancePageAction::EnableCiAutomation)
            })
            .finish();

        let ci_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(
                    Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            Container::new(
                                Text::new_inline("\u{25CF}", appearance.ui_font_family(), 13.)
                                    .with_color(ci_dot_color.into())
                                    .finish(),
                            )
                            .with_margin_right(8.)
                            .finish(),
                        )
                        .with_child(
                            Text::new_inline(ci_status_text, appearance.ui_font_family(), 12.)
                                .with_color(active.into())
                                .finish(),
                        )
                        .finish(),
                )
                .with_child(
                    Container::new(
                        Flex::row()
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_child(ci_refresh_btn)
                            .with_child(Container::new(ci_enable_btn).with_margin_left(6.).finish())
                            .finish(),
                    )
                    .with_margin_top(8.)
                    .finish(),
                )
                .finish(),
            appearance,
        );

        let ci_header = build_sub_header(appearance, "CI / CD", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        // ── Section: Session Context Seed ─────────────────────────────────
        let seed_header = build_sub_header(appearance, "Session Context", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let (seed_dot, seed_text) = match &view.context_seed {
            ContextSeedStatus::Unknown | ContextSeedStatus::Loading => {
                (dim, "Loading context seed\u{2026}".to_string())
            }
            ContextSeedStatus::Loaded { turns, preview } => {
                let s = if *turns == 0 {
                    "No prior session context.".to_string()
                } else if preview.is_empty() {
                    format!("{turns} seed turn(s) ready for next session")
                } else {
                    format!("{turns} seed turn(s)  \u{2014}  {preview}")
                };
                (active, s)
            }
            ContextSeedStatus::Cleared => (dim, "Context cleared.".to_string()),
            ContextSeedStatus::Error(e) => (dim, format!("Unavailable: {e}")),
        };

        let seed_refresh_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.seed_refresh_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(11.),
                padding: Some(Coords::uniform(4.)),
                ..Default::default()
            })
            .with_centered_text_label("Refresh".to_string())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(GovernancePageAction::RefreshContextSeed)
            })
            .finish();

        let seed_clear_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.seed_clear_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(11.),
                padding: Some(Coords::uniform(4.)),
                ..Default::default()
            })
            .with_centered_text_label("Clear Context".to_string())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(GovernancePageAction::ClearContextSeed)
            })
            .finish();

        let seed_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(
                    Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            Container::new(
                                Text::new_inline("\u{25CF}", appearance.ui_font_family(), 13.)
                                    .with_color(seed_dot.into())
                                    .finish(),
                            )
                            .with_margin_right(8.)
                            .finish(),
                        )
                        .with_child(
                            Text::new_inline(seed_text, appearance.ui_font_family(), 12.)
                                .with_color(active.into())
                                .finish(),
                        )
                        .finish(),
                )
                .with_child(
                    Container::new(
                        Flex::row()
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_child(Container::new(seed_refresh_btn).with_margin_right(6.).finish())
                            .with_child(seed_clear_btn)
                            .finish(),
                    )
                    .with_margin_top(8.)
                    .finish(),
                )
                .with_child(Self::dim_label(
                    "Context seed injects prior session summaries into the agent\u{2019}s startup prompt.",
                    appearance,
                ))
                .finish(),
            appearance,
        );

        // ── Section: Dispatch Runs ─────────────────────────────────────────
        let dispatch_header = build_sub_header(appearance, "Multi-Agent Dispatch Runs", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let dispatch_text = match &view.dispatch_runs {
            DispatchRunsStatus::Unknown | DispatchRunsStatus::Loading => {
                "Loading dispatch runs\u{2026}".to_string()
            }
            DispatchRunsStatus::Loaded(runs) => {
                if runs.is_empty() {
                    "No dispatch runs yet.".to_string()
                } else {
                    let recent: Vec<&str> = runs.iter().rev().take(3).map(|s| s.as_str()).collect();
                    format!("{} run(s)  \u{2014}  recent: {}", runs.len(), recent.join(", "))
                }
            }
            DispatchRunsStatus::Error(e) => format!("Unavailable: {e}"),
        };

        let dispatch_refresh_btn = appearance
            .ui_builder()
            .button(
                ButtonVariant::Secondary,
                view.dispatch_refresh_button.clone(),
            )
            .with_style(UiComponentStyles {
                font_size: Some(11.),
                padding: Some(Coords::uniform(4.)),
                ..Default::default()
            })
            .with_centered_text_label("Refresh".to_string())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(GovernancePageAction::RefreshDispatchRuns)
            })
            .finish();

        let dispatch_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(
                    Text::new(dispatch_text, appearance.ui_font_family(), 12.)
                        .with_color(active.into())
                        .soft_wrap(true)
                        .finish(),
                )
                .with_child(
                    Container::new(dispatch_refresh_btn)
                        .with_margin_top(8.)
                        .finish(),
                )
                .with_child(Self::dim_label(
                    "DAG run IDs are stored in .specsmith/dispatch/. Use the serve API for real-time events.",
                    appearance,
                ))
                .finish(),
            appearance,
        );

        // ── Section: Audit Status ──────────────────────────────────────────
        let audit_header = build_sub_header(appearance, "Governance Audit", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let (audit_dot, audit_text) = match &view.audit_status {
            AuditStatusLocal::Unknown | AuditStatusLocal::Loading => {
                (dim, "Loading audit status\u{2026}".to_string())
            }
            AuditStatusLocal::Loaded(resp) => {
                let color = if !resp.ok {
                    dim // pin type; other arms use .into()
                } else if resp.healthy {
                    theme.accent().into_solid().into()
                } else {
                    theme.ui_error_color().into()
                };
                let s = format!(
                    "{}  {}/{} checks passed{}",
                    if resp.healthy { "Healthy" } else { "Issues found" },
                    resp.passed,
                    resp.passed + resp.failed,
                    if resp.fixable > 0 {
                        format!("  ({} fixable)", resp.fixable)
                    } else {
                        String::new()
                    }
                );
                (color, s)
            }
            AuditStatusLocal::Error(e) => (dim, format!("Unavailable: {e}")),
        };

        let audit_refresh_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.audit_refresh_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(11.),
                padding: Some(Coords::uniform(4.)),
                ..Default::default()
            })
            .with_centered_text_label("Refresh".to_string())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(GovernancePageAction::RefreshAuditStatus)
            })
            .finish();

        let audit_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(
                    Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            Container::new(
                                Text::new_inline("\u{25CF}", appearance.ui_font_family(), 13.)
                        .with_color(audit_dot.into())
                                    .finish(),
                            )
                            .with_margin_right(8.)
                            .finish(),
                        )
                        .with_child(
                            Text::new_inline(audit_text, appearance.ui_font_family(), 12.)
                                .with_color(active.into())
                                .finish(),
                        )
                        .finish(),
                )
                .with_child(
                    Container::new(audit_refresh_btn)
                        .with_margin_top(8.)
                        .finish(),
                )
                .finish(),
            appearance,
        );

        // ── Assemble page ─────────────────────────────────────────────────
        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(engine_header)
                    .with_child(engine_card)
                    .with_child(render_separator(appearance))
                    .with_child(seed_header)
                    .with_child(seed_card)
                    .with_child(render_separator(appearance))
                    .with_child(audit_header)
                    .with_child(audit_card)
                    .with_child(render_separator(appearance))
                    .with_child(dispatch_header)
                    .with_child(dispatch_card)
                    .with_child(render_separator(appearance))
                    .with_child(ctx_win_header)
                    .with_child(ctx_win_card)
                    .with_child(Container::new(optimize_btn).with_margin_top(8.).finish())
                    .with_child(render_separator(appearance))
                    .with_child(ci_header)
                    .with_child(ci_card)
                    .with_child(render_separator(appearance))
                    .with_child(updater_header)
                    .with_child(updater_card)
                    .with_child(render_separator(appearance))
                    .with_child(links_header)
                    .with_child(links_card)
                    .finish(),
            )
            .with_max_width(720.)
            .finish(),
        )
        .with_uniform_padding(28.)
        .finish()
    }
}

// ---------------------------------------------------------------------------
// Settings metadata
// ---------------------------------------------------------------------------

impl SettingsPageMeta for GovernancePageView {
    fn section() -> SettingsSection {
        SettingsSection::Governance
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn on_page_selected(&mut self, _: bool, ctx: &mut ViewContext<Self>) {
        // Refresh engine health + channel when user navigates to this page.
        // (Active Project controls are in the Governance tools panel.)
        self.check_health(ctx);
        self.refresh_channel(ctx);
        ctx.notify();
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<GovernancePageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<GovernancePageView>) -> Self {
        SettingsPageViewHandle::Governance(view_handle)
    }
}
