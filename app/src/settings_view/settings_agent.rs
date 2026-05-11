//! Settings Assistant — small, collapsible AI agent panel for the settings sidebar.
//!
//! Provides quick-action buttons (Audit, Compliance, ESDB status, Build Skill) that
//! invoke `specsmith agent ask` and display the response inline.
//!
//! The panel is collapsed by default; clicking the "Assistant" header row toggles it.

use crate::appearance::Appearance;
use warpui::{
    elements::{
        ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element, Flex, Hoverable,
        MouseStateHandle, ParentElement, Radius, Text,
    },
    platform::Cursor,
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
    },
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext,
};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
enum AgentQueryStatus {
    #[default]
    Idle,
    Running,
    Done(String),
    Error(String),
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SettingsAgentAction {
    /// Toggle the panel collapsed / expanded.
    Toggle,
    /// Run `specsmith agent ask <prompt> --json-output`.
    Ask(String),
    // Quick-action shortcuts
    QuickAudit,
    QuickCompliance,
    QuickEsdb,
    QuickSession,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct SettingsAgentView {
    expanded: bool,
    query_status: AgentQueryStatus,
    project_dir: Option<std::path::PathBuf>,
    toggle_button_ms: MouseStateHandle,
    audit_btn_ms: MouseStateHandle,
    compliance_btn_ms: MouseStateHandle,
    esdb_btn_ms: MouseStateHandle,
    session_btn_ms: MouseStateHandle,
}

impl SettingsAgentView {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        SettingsAgentView {
            expanded: false,
            query_status: AgentQueryStatus::Idle,
            project_dir: std::env::current_dir().ok(),
            toggle_button_ms: MouseStateHandle::default(),
            audit_btn_ms: MouseStateHandle::default(),
            compliance_btn_ms: MouseStateHandle::default(),
            esdb_btn_ms: MouseStateHandle::default(),
            session_btn_ms: MouseStateHandle::default(),
        }
    }

    fn ask(&mut self, prompt: impl Into<String>, ctx: &mut ViewContext<Self>) {
        let prompt = prompt.into();
        self.query_status = AgentQueryStatus::Running;
        ctx.notify();

        let project_dir = self.project_dir.clone();
        ctx.spawn(
            async move {
                let run = |prog: &str, args: &[&str]| -> Result<String, String> {
                    let mut c = std::process::Command::new(prog);
                    c.args(args);
                    if let Some(dir) = &project_dir {
                        c.current_dir(dir);
                    }
                    c.env("SPECSMITH_NO_AUTO_UPDATE", "1");
                    c.env("SPECSMITH_PYPI_CHECKED", "1");
                    c.output()
                        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                        .map_err(|e| e.to_string())
                };
                let py_args = ["-m", "specsmith", "agent", "ask", &prompt, "--json-output"];
                run("py", &py_args)
                    .or_else(|_| run("specsmith", &["agent", "ask", &prompt, "--json-output"]))
                    .map_err(|e| format!("specsmith not found: {e}"))
            },
            |me, result: Result<String, String>, ctx| {
                me.query_status = match result {
                    Err(e) => AgentQueryStatus::Error(e.chars().take(120).collect()),
                    Ok(text) => {
                        // Try to parse {"reply": "..."} JSON; fall back to raw text
                        let reply = serde_json::from_str::<serde_json::Value>(&text)
                            .ok()
                            .and_then(|v| v.get("reply")?.as_str().map(|s| s.to_owned()))
                            .unwrap_or_else(|| text.lines().next().unwrap_or("").to_owned());
                        AgentQueryStatus::Done(reply.chars().take(200).collect())
                    }
                };
                ctx.notify();
            },
        );
    }
}

impl Entity for SettingsAgentView {
    type Event = ();
}

impl TypedActionView for SettingsAgentView {
    type Action = SettingsAgentAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            SettingsAgentAction::Toggle => {
                self.expanded = !self.expanded;
                ctx.notify();
            }
            SettingsAgentAction::Ask(prompt) => self.ask(prompt.clone(), ctx),
            SettingsAgentAction::QuickAudit => self.ask("audit health governance", ctx),
            SettingsAgentAction::QuickCompliance => self.ask("compliance coverage gaps", ctx),
            SettingsAgentAction::QuickEsdb => self.ask("esdb database status records", ctx),
            SettingsAgentAction::QuickSession => self.ask("session status phase project", ctx),
        }
    }
}

impl View for SettingsAgentView {
    fn ui_name() -> &'static str {
        "SettingsAgentView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color();
        let _accent = theme.accent().into_solid();

        // ── Header row (always visible) ───────────────────────────────────
        let chevron = if self.expanded {
            "\u{25BE}"
        } else {
            "\u{25B8}"
        };
        let header = Hoverable::new(self.toggle_button_ms.clone(), move |_| {
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Container::new(
                        Text::new_inline(chevron.to_string(), appearance.ui_font_family(), 10.)
                            .with_color(dim.into())
                            .finish(),
                    )
                    .with_margin_right(6.)
                    .finish(),
                )
                .with_child(
                    Text::new_inline(
                        "\u{2728}  Settings Assistant".to_string(),
                        appearance.ui_font_family(),
                        11.,
                    )
                    .with_color(dim.into())
                    .finish(),
                )
                .finish()
        })
        .on_click(|ctx, _, _| ctx.dispatch_typed_action(SettingsAgentAction::Toggle))
        .with_cursor(Cursor::PointingHand)
        .finish();

        let header_row = Container::new(header)
            .with_padding_top(8.)
            .with_padding_bottom(8.)
            .with_padding_left(12.)
            .with_padding_right(12.)
            .finish();

        if !self.expanded {
            return header_row;
        }

        // ── Quick-action buttons ──────────────────────────────────────────
        let make_btn = |label: &str,
                        ms: MouseStateHandle,
                        action: SettingsAgentAction,
                        appearance: &Appearance|
         -> Box<dyn Element> {
            appearance
                .ui_builder()
                .button(ButtonVariant::Secondary, ms)
                .with_style(UiComponentStyles {
                    font_size: Some(10.),
                    padding: Some(Coords::uniform(4.)),
                    ..Default::default()
                })
                .with_centered_text_label(label.to_owned())
                .build()
                .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
                .finish()
        };

        let btn_row = Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(make_btn(
                    "Audit",
                    self.audit_btn_ms.clone(),
                    SettingsAgentAction::QuickAudit,
                    appearance,
                ))
                .with_child(
                    Container::new(make_btn(
                        "Compliance",
                        self.compliance_btn_ms.clone(),
                        SettingsAgentAction::QuickCompliance,
                        appearance,
                    ))
                    .with_margin_left(4.)
                    .finish(),
                )
                .with_child(
                    Container::new(make_btn(
                        "ESDB",
                        self.esdb_btn_ms.clone(),
                        SettingsAgentAction::QuickEsdb,
                        appearance,
                    ))
                    .with_margin_left(4.)
                    .finish(),
                )
                .with_child(
                    Container::new(make_btn(
                        "Session",
                        self.session_btn_ms.clone(),
                        SettingsAgentAction::QuickSession,
                        appearance,
                    ))
                    .with_margin_left(4.)
                    .finish(),
                )
                .finish(),
        )
        .with_padding_left(12.)
        .with_padding_right(12.)
        .with_padding_bottom(6.)
        .finish();

        // ── Output area ───────────────────────────────────────────────────
        let output_elem: Option<Box<dyn Element>> = match &self.query_status {
            AgentQueryStatus::Idle => None,
            AgentQueryStatus::Running => Some(
                Text::new(
                    "Asking\u{2026}".to_string(),
                    appearance.ui_font_family(),
                    10.,
                )
                .with_color(dim.into())
                .finish(),
            ),
            AgentQueryStatus::Done(reply) => Some(
                Container::new(
                    Text::new(reply.clone(), appearance.ui_font_family(), 11.)
                        .with_color(active.into())
                        .soft_wrap(true)
                        .finish(),
                )
                .with_background(theme.surface_1())
                .with_uniform_padding(8.)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
                .finish(),
            ),
            AgentQueryStatus::Error(msg) => Some(
                Text::new(msg.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(theme.ui_error_color().into())
                    .soft_wrap(true)
                    .finish(),
            ),
        };

        let mut body_col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(btn_row);

        if let Some(out) = output_elem {
            body_col.add_child(
                Container::new(out)
                    .with_padding_left(12.)
                    .with_padding_right(12.)
                    .with_padding_bottom(8.)
                    .finish(),
            );
        }

        // Hint text
        body_col.add_child(
            Container::new(
                Text::new(
                    "Powered by specsmith agent ask".to_string(),
                    appearance.ui_font_family(),
                    9.,
                )
                .with_color(dim.into())
                .finish(),
            )
            .with_padding_left(12.)
            .with_padding_bottom(8.)
            .finish(),
        );

        let accent_top = Container::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(header_row)
                .with_child(body_col.finish())
                .finish(),
        )
        .with_background(theme.surface_1())
        .finish();

        ConstrainedBox::new(accent_top)
            .with_max_width(248.)
            .finish()
    }
}
