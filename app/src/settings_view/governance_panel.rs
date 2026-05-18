//! Governance operational dashboard — shown in the left-panel toolbelt (GovernancePanelView).
//!
//! This is the ACTION-oriented counterpart to `governance_page.rs` (settings config).
//! Sections:
//!  1. Session status  — specsmith session info: project, phase, health %, compliance %
//!  2. Active project  — Detect / Audit / Init / Sync / Fix buttons + output
//!  3. Epistemic score — health_score % with colour coding

use super::settings_page::{build_sub_header, render_separator, HEADER_PADDING};
use crate::appearance::Appearance;
use crate::governance_project::GovernanceProjectState;
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
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext,
};

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
struct SessionInfo {
    project_name: String,
    phase_emoji: String,
    phase_label: String,
    phase_pct: u32,
    health_score: u32,
    compliance_score: u32,
    governed: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum SessionStatus {
    Unknown,
    Loading,
    Loaded(SessionInfo),
    Error(String),
}

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
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GovernancePanelAction {
    RefreshSession,
    DetectProject,
    RunAudit,
    InitProject,
    SyncProject,
    FixIssues,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct GovernancePanelView {
    session: SessionStatus,
    project_dir: Option<PathBuf>,
    project_has_specsmith: bool,
    project_action: ProjectActionStatus,
    refresh_button: MouseStateHandle,
    detect_button: MouseStateHandle,
    audit_button: MouseStateHandle,
    init_button: MouseStateHandle,
    sync_button: MouseStateHandle,
    fix_button: MouseStateHandle,
}

impl GovernancePanelView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(
            &GovernanceProjectState::handle(ctx),
            |me, _, _event, ctx| {
                let state = GovernanceProjectState::as_ref(ctx);
                me.project_dir = state.active_dir.clone();
                me.project_has_specsmith = state.has_specsmith;
                ctx.notify();
            },
        );

        let initial_dir = std::env::current_dir().ok();
        let initial_has_specsmith = initial_dir
            .as_ref()
            .map(|d| d.join(".specsmith").is_dir())
            .unwrap_or(false);

        let mut view = GovernancePanelView {
            session: SessionStatus::Unknown,
            project_dir: initial_dir,
            project_has_specsmith: initial_has_specsmith,
            project_action: ProjectActionStatus::Idle,
            refresh_button: MouseStateHandle::default(),
            detect_button: MouseStateHandle::default(),
            audit_button: MouseStateHandle::default(),
            init_button: MouseStateHandle::default(),
            sync_button: MouseStateHandle::default(),
            fix_button: MouseStateHandle::default(),
        };
        view.refresh_session(ctx);
        view
    }

    fn refresh_session(&mut self, ctx: &mut ViewContext<Self>) {
        self.session = SessionStatus::Loading;
        ctx.notify();

        let project_dir = self
            .project_dir
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        ctx.spawn(
            async move {
                let run = |prog: &str, args: &[&str]| -> Result<std::process::Output, String> {
                    let mut c = std::process::Command::new(prog);
                    c.args(args);
                    c.current_dir(&project_dir);
                    c.env("SPECSMITH_NO_AUTO_UPDATE", "1");
                    c.env("SPECSMITH_PYPI_CHECKED", "1");
                    c.output().map_err(|e| e.to_string())
                };
                let out = run(
                    "py",
                    &["-m", "specsmith", "session", "info", "--json-output"],
                )
                .or_else(|_| run("specsmith", &["session", "info", "--json-output"]))
                .map_err(|e| format!("specsmith not found: {e}"))?;
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                Ok(text)
            },
            |me, result: Result<String, String>, ctx| {
                me.session = match result {
                    Err(e) => SessionStatus::Error(e),
                    Ok(text) => {
                        // Best-effort JSON parse
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            SessionStatus::Loaded(SessionInfo {
                                project_name: val
                                    .get("project_name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_owned(),
                                phase_emoji: val
                                    .get("phase_emoji")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("\u{1F4CB}")
                                    .to_owned(),
                                phase_label: val
                                    .get("phase_label")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_owned(),
                                phase_pct: val
                                    .get("phase_readiness_pct")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0) as u32,
                                health_score: val
                                    .get("health_score")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0)
                                    as u32,
                                compliance_score: val
                                    .get("compliance_score")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0)
                                    as u32,
                                governed: val
                                    .get("is_governed")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false),
                            })
                        } else {
                            SessionStatus::Error(
                                "specsmith not running or no project found".to_owned(),
                            )
                        }
                    }
                };
                ctx.notify();
            },
        );
    }

    fn run_specsmith_cmd(&mut self, cmd: &'static str, ctx: &mut ViewContext<Self>) {
        self.project_action = ProjectActionStatus::Running {
            action: cmd.to_owned(),
        };
        ctx.notify();

        let project_dir = self.project_dir.clone();
        ctx.spawn(
            async move {
                let run_with =
                    |prog: &str, args: &[&str]| -> Result<std::process::Output, String> {
                        let mut c = std::process::Command::new(prog);
                        c.args(args);
                        c.arg(cmd);
                        if let Some(dir) = &project_dir {
                            c.current_dir(dir);
                        }
                        c.env("SPECSMITH_NO_AUTO_UPDATE", "1");
                        c.env("SPECSMITH_PYPI_CHECKED", "1");
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
                if let Some(dir) = &me.project_dir {
                    me.project_has_specsmith = dir.join(".specsmith").is_dir();
                }
                ctx.notify();
            },
        );
    }
}

impl Entity for GovernancePanelView {
    type Event = ();
}

impl TypedActionView for GovernancePanelView {
    type Action = GovernancePanelAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            GovernancePanelAction::RefreshSession => self.refresh_session(ctx),
            GovernancePanelAction::DetectProject => {
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
            GovernancePanelAction::RunAudit => self.run_specsmith_cmd("audit", ctx),
            GovernancePanelAction::InitProject => self.run_specsmith_cmd("init", ctx),
            GovernancePanelAction::SyncProject => self.run_specsmith_cmd("sync", ctx),
            GovernancePanelAction::FixIssues => {
                // Run `specsmith audit --fix`
                self.project_action = ProjectActionStatus::Running {
                    action: "fix".to_owned(),
                };
                ctx.notify();
                let project_dir = self.project_dir.clone();
                ctx.spawn(
                    async move {
                        let run_with =
                            |prog: &str, args: &[&str]| -> Result<std::process::Output, String> {
                                let mut c = std::process::Command::new(prog);
                                c.args(args);
                                if let Some(dir) = &project_dir {
                                    c.current_dir(dir);
                                }
                                c.env("SPECSMITH_NO_AUTO_UPDATE", "1");
                                c.env("SPECSMITH_PYPI_CHECKED", "1");
                                c.output().map_err(|e| e.to_string())
                            };
                        let out = run_with("py", &["-m", "specsmith", "audit", "--fix"])
                            .or_else(|_| run_with("specsmith", &["audit", "--fix"]))
                            .map_err(|e| format!("specsmith not found: {e}"))?;
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        Ok(format!("{stdout}{stderr}"))
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
                        ctx.notify();
                    },
                );
            }
        }
    }
}

impl View for GovernancePanelView {
    fn ui_name() -> &'static str {
        "GovernancePanel"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        Self::render_content(self, appearance)
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

impl GovernancePanelView {
    fn action_button(
        label: impl Into<String>,
        variant: ButtonVariant,
        mouse_state: MouseStateHandle,
        action: GovernancePanelAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        appearance
            .ui_builder()
            .button(variant, mouse_state)
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

    fn render_content(view: &GovernancePanelView, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color();
        let accent = theme.accent().into_solid();

        // ── Section 1: Session status ────────────────────────────────────
        let session_header = build_sub_header(appearance, "Session", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let session_card = match &view.session {
            SessionStatus::Unknown | SessionStatus::Loading => Self::card(
                Text::new(
                    "Loading session\u{2026}".to_string(),
                    appearance.ui_font_family(),
                    12.,
                )
                .with_color(dim.into())
                .finish(),
                appearance,
            ),
            SessionStatus::Error(msg) => Self::card(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(
                        Text::new(
                            "Session unavailable".to_string(),
                            appearance.ui_font_family(),
                            12.,
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new(
                                msg.chars().take(120).collect::<String>(),
                                appearance.monospace_font_family(),
                                10.,
                            )
                            .with_color(theme.ui_error_color().into())
                            .soft_wrap(true)
                            .finish(),
                        )
                        .with_margin_top(4.)
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new(
                                "Run specsmith governance-serve to start".to_string(),
                                appearance.monospace_font_family(),
                                10.,
                            )
                            .with_color(dim.into())
                            .finish(),
                        )
                        .with_margin_top(4.)
                        .finish(),
                    )
                    .finish(),
                appearance,
            ),
            SessionStatus::Loaded(info) => {
                // Health score colour
                let health_color = if info.health_score >= 80 {
                    accent.into()
                } else if info.health_score >= 50 {
                    active
                } else {
                    theme.ui_error_color().into()
                };
                let compliance_color = if info.compliance_score >= 80 {
                    accent.into()
                } else if info.compliance_score >= 50 {
                    active
                } else {
                    theme.ui_error_color().into()
                };

                let gov_dot = if info.governed {
                    "\u{25CF}"
                } else {
                    "\u{25CB}"
                };
                let gov_color = if info.governed { accent.into() } else { dim };

                Self::card(
                    Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .with_child(
                            Flex::row()
                                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                .with_child(
                                    Text::new_inline(
                                        gov_dot.to_string(),
                                        appearance.ui_font_family(),
                                        13.,
                                    )
                                    .with_color(gov_color.into())
                                    .finish(),
                                )
                                .with_child(
                                    Container::new(
                                        Text::new_inline(
                                            format!("  {}", info.project_name),
                                            appearance.ui_font_family(),
                                            13.,
                                        )
                                        .with_color(active.into())
                                        .finish(),
                                    )
                                    .finish(),
                                )
                                .finish(),
                        )
                        .with_child(
                            Container::new(
                                Text::new(
                                    format!(
                                        "{} {} \u{2014} {}%",
                                        info.phase_emoji, info.phase_label, info.phase_pct
                                    ),
                                    appearance.ui_font_family(),
                                    12.,
                                )
                                .with_color(dim.into())
                                .finish(),
                            )
                            .with_margin_top(6.)
                            .finish(),
                        )
                        .with_child(
                            Container::new(
                                Flex::row()
                                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                    .with_child(
                                        Expanded::new(
                                            1.,
                                            Text::new_inline(
                                                "Health".to_string(),
                                                appearance.ui_font_family(),
                                                11.,
                                            )
                                            .with_color(dim.into())
                                            .finish(),
                                        )
                                        .finish(),
                                    )
                                    .with_child(
                                        Text::new_inline(
                                            format!("{}%", info.health_score),
                                            appearance.monospace_font_family(),
                                            13.,
                                        )
                                        .with_color(health_color.into())
                                        .finish(),
                                    )
                                    .finish(),
                            )
                            .with_margin_top(8.)
                            .finish(),
                        )
                        .with_child(
                            Container::new(
                                Flex::row()
                                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                    .with_child(
                                        Expanded::new(
                                            1.,
                                            Text::new_inline(
                                                "Compliance".to_string(),
                                                appearance.ui_font_family(),
                                                11.,
                                            )
                                            .with_color(dim.into())
                                            .finish(),
                                        )
                                        .finish(),
                                    )
                                    .with_child(
                                        Text::new_inline(
                                            format!("{}%", info.compliance_score),
                                            appearance.monospace_font_family(),
                                            13.,
                                        )
                                        .with_color(compliance_color.into())
                                        .finish(),
                                    )
                                    .finish(),
                            )
                            .with_margin_top(4.)
                            .finish(),
                        )
                        .finish(),
                    appearance,
                )
            }
        };

        // ── Section 2: Active project actions ────────────────────────────
        let project_header = build_sub_header(appearance, "Active Project", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let (proj_dot_color, proj_status_text) = match &view.project_dir {
            None => (dim, "No project detected".to_string()),
            Some(dir) => {
                if view.project_has_specsmith {
                    (
                        accent.into(),
                        format!("\u{2714}  .specsmith/  \u{2014}  {}", dir.display()),
                    )
                } else {
                    (
                        dim,
                        format!("\u{26A0}  No .specsmith/  \u{2014}  {}", dir.display()),
                    )
                }
            }
        };

        let proj_dot = Text::new_inline("\u{25CF}", appearance.ui_font_family(), 13.)
            .with_color(proj_dot_color.into())
            .finish();

        let proj_status_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Container::new(proj_dot).with_margin_right(6.).finish())
            .with_child(
                Text::new(proj_status_text, appearance.monospace_font_family(), 10.)
                    .with_color(dim.into())
                    .soft_wrap(true)
                    .finish(),
            )
            .finish();

        let action_buttons = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(Self::action_button(
                    "Detect",
                    ButtonVariant::Secondary,
                    view.detect_button.clone(),
                    GovernancePanelAction::DetectProject,
                    appearance,
                ))
                .with_margin_right(5.)
                .finish(),
            )
            .with_child(
                Container::new(Self::action_button(
                    "Audit",
                    ButtonVariant::Secondary,
                    view.audit_button.clone(),
                    GovernancePanelAction::RunAudit,
                    appearance,
                ))
                .with_margin_right(5.)
                .finish(),
            )
            .with_child(
                Container::new(Self::action_button(
                    "Fix",
                    ButtonVariant::Accent,
                    view.fix_button.clone(),
                    GovernancePanelAction::FixIssues,
                    appearance,
                ))
                .with_margin_right(5.)
                .finish(),
            )
            .with_child(
                Container::new(Self::action_button(
                    "Init",
                    ButtonVariant::Secondary,
                    view.init_button.clone(),
                    GovernancePanelAction::InitProject,
                    appearance,
                ))
                .with_margin_right(5.)
                .finish(),
            )
            .with_child(Self::action_button(
                "Sync",
                ButtonVariant::Secondary,
                view.sync_button.clone(),
                GovernancePanelAction::SyncProject,
                appearance,
            ))
            .finish();

        let action_output: Option<Box<dyn Element>> = match &view.project_action {
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
            .with_child(proj_status_row)
            .with_child(Container::new(action_buttons).with_margin_top(10.).finish());

        if let Some(out) = action_output {
            project_col.add_child(Container::new(out).with_margin_top(8.).finish());
        }

        let project_card = Self::card(project_col.finish(), appearance);

        // ── Refresh button ───────────────────────────────────────────────
        let refresh_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.refresh_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label("Refresh session".to_string())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(GovernancePanelAction::RefreshSession);
            })
            .finish();

        // ── Assemble ─────────────────────────────────────────────────────
        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(session_header)
                    .with_child(session_card)
                    .with_child(render_separator(appearance))
                    .with_child(project_header)
                    .with_child(project_card)
                    .with_child(Container::new(refresh_btn).with_margin_top(8.).finish())
                    .finish(),
            )
            .with_max_width(720.)
            .finish(),
        )
        .with_uniform_padding(20.)
        .finish()
    }
}
