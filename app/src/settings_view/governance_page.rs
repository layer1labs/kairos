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
use crate::governance_project::GovernanceProjectState;
use kairos_governance::{GovernanceClient, GovernanceConfig};
use std::path::PathBuf;
use warpui::{
    elements::{
        ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element, Expanded, Flex,
        MouseStateHandle, ParentElement, Radius, Text,
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

// ---------------------------------------------------------------------------
// Project action state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
enum ProjectActionStatus {
    #[default]
    Idle,
    Running {
        action: String,
    },
    Output {
        lines: String,
    },
    Error {
        message: String,
    },
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GovernancePageAction {
    CheckForSpecsmithUpdate,
    UpdateSpecsmith,
    /// Re-detect the active project directory.
    DetectProject,
    /// Run `specsmith audit` in the detected project directory.
    RunAudit,
    /// Run `specsmith init` to scaffold governance for the project.
    InitProject,
    /// Run `specsmith sync` to regenerate machine state JSON.
    SyncProject,
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
    /// Active project directory (from GovernanceProjectState or auto-detected).
    project_dir: Option<PathBuf>,
    /// Whether `.specsmith/` exists in the active project directory.
    project_has_specsmith: bool,
    /// Status of the last project action (audit / init / sync).
    project_action: ProjectActionStatus,
    detect_project_button: MouseStateHandle,
    run_audit_button: MouseStateHandle,
    init_project_button: MouseStateHandle,
    sync_project_button: MouseStateHandle,
}

impl GovernancePageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        // Subscribe to GovernanceProjectState so we update when workspace dir changes.
        ctx.subscribe_to_model(
            &GovernanceProjectState::handle(ctx),
            |me, _, _event, ctx| {
                let state = GovernanceProjectState::as_ref(ctx);
                me.project_dir = state.active_dir.clone();
                me.project_has_specsmith = state.has_specsmith;
                ctx.notify();
            },
        );

        // Bootstrap initial project dir from the process working directory.
        let initial_dir = std::env::current_dir().ok();
        let initial_has_specsmith = initial_dir
            .as_ref()
            .map(|d| d.join(".specsmith").is_dir())
            .unwrap_or(false);

        let mut view = GovernancePageView {
            page: PageType::new_monolith(GovernancePageWidget::default(), None, false),
            health: HealthStatus::Unknown,
            updater: UpdaterStatus::Idle,
            check_update_button: MouseStateHandle::default(),
            do_update_button: MouseStateHandle::default(),
            project_dir: initial_dir,
            project_has_specsmith: initial_has_specsmith,
            project_action: ProjectActionStatus::Idle,
            detect_project_button: MouseStateHandle::default(),
            run_audit_button: MouseStateHandle::default(),
            init_project_button: MouseStateHandle::default(),
            sync_project_button: MouseStateHandle::default(),
        };
        view.check_health(ctx);
        view
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

    /// Run a specsmith command (audit / init / sync) in the detected project directory.
    fn run_specsmith_cmd(&mut self, cmd: &'static str, ctx: &mut ViewContext<Self>) {
        self.project_action = ProjectActionStatus::Running {
            action: cmd.to_owned(),
        };
        ctx.notify();

        let project_dir = self.project_dir.clone();
        ctx.spawn(
            async move {
                // Try `py -m specsmith` first (Windows/pipx), then bare `specsmith` (Unix).
                let run_with =
                    |prog: &str, args: &[&str]| -> Result<std::process::Output, String> {
                        let mut c = std::process::Command::new(prog);
                        c.args(args);
                        c.arg(cmd);
                        if let Some(dir) = &project_dir {
                            c.current_dir(dir);
                        }
                        c.output().map_err(|e| e.to_string())
                    };

                let out = run_with("py", &["-m", "specsmith"])
                    .or_else(|_| run_with("specsmith", &[]))
                    .map_err(|e| format!("specsmith not found: {e}"))?;

                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                if out.status.success() {
                    Ok(stdout)
                } else {
                    Err(format!("{stdout}\n{stderr}"))
                }
            },
            |me, result: Result<String, String>, ctx| {
                me.project_action = match result {
                    Ok(output) => ProjectActionStatus::Output {
                        lines: output.lines().take(12).collect::<Vec<_>>().join("\n"),
                    },
                    Err(e) => ProjectActionStatus::Error {
                        message: e.chars().take(200).collect::<String>(),
                    },
                };
                // After init, re-detect .specsmith/ presence.
                if let Some(dir) = &me.project_dir {
                    me.project_has_specsmith = dir.join(".specsmith").is_dir();
                }
                ctx.notify();
            },
        );
    }

    /// Runs `pipx upgrade specsmith` in a subprocess and updates `updater` state.
    fn run_pipx_upgrade(&mut self, ctx: &mut ViewContext<Self>) {
        self.updater = UpdaterStatus::Updating;
        ctx.notify();
        ctx.spawn(
            async move {
                let out = std::process::Command::new("pipx")
                    .args(["upgrade", "specsmith"])
                    .output()
                    .map_err(|e| format!("pipx not found: {e:#}"))?;
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

    /// Runs `pipx list --short` to get the installed specsmith version.
    fn check_for_update(&mut self, ctx: &mut ViewContext<Self>) {
        self.updater = UpdaterStatus::Checking;
        ctx.notify();
        ctx.spawn(
            async move {
                let out = std::process::Command::new("pipx")
                    .args(["list", "--short"])
                    .output()
                    .map_err(|e| format!("pipx not found: {e:#}"))?;
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                Ok(stdout)
            },
            |me, result: Result<String, String>, ctx| {
                me.updater = match result {
                    Ok(output) => {
                        let current = output
                            .lines()
                            .find(|l| l.to_lowercase().contains("specsmith"))
                            .and_then(|l| l.split_whitespace().nth(1))
                            .map(|v| v.to_owned())
                            .unwrap_or_else(|| "unknown".to_owned());
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
            GovernancePageAction::DetectProject => {
                let state = GovernanceProjectState::as_ref(ctx);
                if let Some(dir) = state.active_dir.clone() {
                    self.project_has_specsmith = dir.join(".specsmith").is_dir();
                    self.project_dir = Some(dir);
                } else if let Ok(cwd) = std::env::current_dir() {
                    self.project_has_specsmith = cwd.join(".specsmith").is_dir();
                    self.project_dir = Some(cwd);
                }
                self.project_action = ProjectActionStatus::Idle;
                ctx.notify();
            }
            GovernancePageAction::RunAudit => self.run_specsmith_cmd("audit", ctx),
            GovernancePageAction::InitProject => self.run_specsmith_cmd("init", ctx),
            GovernancePageAction::SyncProject => self.run_specsmith_cmd("sync", ctx),
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
        "governance specsmith local ai engine BYOE endpoint port 7700 update pipx audit init project sync"
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

        // ── Section 2: Project context ────────────────────────────────────
        let project_header = build_sub_header(appearance, "Active Project", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let (project_dot_color, project_path_text, project_status_text) = match &view.project_dir {
            None => (dim, "No project detected".to_string(), "".to_string()),
            Some(dir) => {
                let path_str = dir.display().to_string();
                if view.project_has_specsmith {
                    (
                        theme.accent().into_solid().into(),
                        path_str,
                        "\u{2714}  .specsmith/ found \u{2014} governance active".to_string(),
                    )
                } else {
                    (
                        dim,
                        path_str,
                        "\u{26A0}  No .specsmith/ \u{2014} click Init to set up governance"
                            .to_string(),
                    )
                }
            }
        };

        let project_dot = Text::new_inline("\u{25CF}", appearance.ui_font_family(), 13.)
            .with_color(project_dot_color.into())
            .finish();

        let project_status_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Container::new(project_dot).with_margin_right(8.).finish())
            .with_child(
                Text::new_inline(project_status_text, appearance.ui_font_family(), 12.)
                    .with_color(active.into())
                    .finish(),
            )
            .finish();

        let project_buttons = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(Self::action_button(
                    "Detect",
                    view.detect_project_button.clone(),
                    GovernancePageAction::DetectProject,
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::action_button(
                    "Audit",
                    view.run_audit_button.clone(),
                    GovernancePageAction::RunAudit,
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::action_button(
                    "Init",
                    view.init_project_button.clone(),
                    GovernancePageAction::InitProject,
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(Self::action_button(
                "Sync",
                view.sync_project_button.clone(),
                GovernancePageAction::SyncProject,
                appearance,
            ))
            .finish();

        let action_output_elem: Option<Box<dyn Element>> = match &view.project_action {
            ProjectActionStatus::Idle => None,
            ProjectActionStatus::Running { action } => Some(
                Text::new(
                    format!("Running specsmith {action}\u{2026}"),
                    appearance.ui_font_family(),
                    11.,
                )
                .with_color(dim.into())
                .finish(),
            ),
            ProjectActionStatus::Output { lines } => Some(
                Text::new(lines.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(active.into())
                    .soft_wrap(true)
                    .finish(),
            ),
            ProjectActionStatus::Error { message } => Some(
                Text::new(message.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(theme.ui_error_color().into())
                    .soft_wrap(true)
                    .finish(),
            ),
        };

        let mut project_col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(project_status_row)
            .with_child(Self::dim_label(project_path_text, appearance))
            .with_child(
                Container::new(project_buttons)
                    .with_margin_top(10.)
                    .finish(),
            );

        if let Some(out) = action_output_elem {
            project_col.add_child(Container::new(out).with_margin_top(8.).finish());
        }

        let project_card = Self::card(project_col.finish(), appearance);

        // ── Section 3: specsmith updater ──────────────────────────────────
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
            UpdaterStatus::Updating => ("Updating via pipx\u{2026}".to_string(), dim),
            UpdaterStatus::Updated { version } => (
                format!("Updated to specsmith {version}"),
                theme.accent().into_solid().into(),
            ),
            UpdaterStatus::Error { message } => (format!("Error: {message}"), dim),
        };

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
                                "Update (pipx upgrade specsmith)",
                                view.do_update_button.clone(),
                                GovernancePageAction::UpdateSpecsmith,
                                appearance,
                            ))
                            .finish(),
                    )
                    .with_margin_top(10.)
                    .finish(),
                )
                .with_child(
                    Container::new(
                        Text::new(
                            "Managed via pipx. To install specsmith for the first time: pipx install specsmith"
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

        // ── Section 4: Bug report links ───────────────────────────────────
        let links_header = build_sub_header(appearance, "Report Bugs", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let link_row = |label: &str, url: &str| -> Box<dyn Element> {
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Expanded::new(
                        1.,
                        Container::new(
                            Text::new_inline(label.to_string(), appearance.ui_font_family(), 12.)
                                .with_color(dim.into())
                                .finish(),
                        )
                        .with_margin_right(12.)
                        .finish(),
                    )
                    .finish(),
                )
                .with_child(
                    Text::new_inline(
                        format!("\u{2192}  {url}"),
                        appearance.monospace_font_family(),
                        11.,
                    )
                    .with_color(theme.accent().into_solid().into())
                    .finish(),
                )
                .finish()
        };

        let links_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(link_row(
                    "Governance / specsmith bugs",
                    "github.com/BitConcepts/specsmith",
                ))
                .with_child(
                    Container::new(link_row(
                        "Kairos terminal bugs",
                        "github.com/BitConcepts/kairos",
                    ))
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
                    .with_child(project_header)
                    .with_child(project_card)
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
        // Refresh health and re-detect project dir when user navigates to this page.
        self.check_health(ctx);
        let state = GovernanceProjectState::as_ref(ctx);
        if let Some(dir) = state.active_dir.clone() {
            self.project_has_specsmith = dir.join(".specsmith").is_dir();
            self.project_dir = Some(dir);
        } else if let Ok(cwd) = std::env::current_dir() {
            self.project_has_specsmith = cwd.join(".specsmith").is_dir();
            self.project_dir = Some(cwd);
        }
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
