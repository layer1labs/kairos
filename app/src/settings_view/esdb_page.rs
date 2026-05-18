//! ESDB (Epistemic State Database) dashboard — status, record counts, and database management.
//!
//! Sections:
//!  1. Status        — backend, record count, chain integrity
//!  2. Record counts — facts, hypotheses, decisions, risks, requirements, tests
//!  3. DB Management — Export, Import, Backup, Rollback, Compact, Migrate, Replay
//!  4. Actions bar   — Refresh + management buttons

use super::{
    settings_page::{
        build_sub_header, render_separator, MatchData, PageType, SettingsPageEvent,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use crate::themes::theme::Fill;
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
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
struct EsdbData {
    available: bool,
    backend: String,
    record_count: usize,
    chain_valid: bool,
    // Record type breakdown
    requirements: usize,
    testcases: usize,
    facts: usize,
    hypotheses: usize,
    decisions: usize,
    risks: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum EsdbStatus {
    Unknown,
    Loading,
    Loaded(EsdbData),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
enum OpStatus {
    #[default]
    Idle,
    Running(String),
    Done(String),
    Error(String),
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum EsdbPageAction {
    Refresh,
    Export,
    Import,
    Backup,
    Rollback,
    Compact,
    Migrate,
    Replay,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct EsdbPageView {
    page: PageType<Self>,
    status: EsdbStatus,
    op_status: OpStatus,
    project_dir: Option<PathBuf>,
    refresh_button: MouseStateHandle,
    export_button: MouseStateHandle,
    import_button: MouseStateHandle,
    backup_button: MouseStateHandle,
    rollback_button: MouseStateHandle,
    compact_button: MouseStateHandle,
    migrate_button: MouseStateHandle,
    replay_button: MouseStateHandle,
}

impl EsdbPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let initial_dir = std::env::current_dir().ok();
        let mut view = EsdbPageView {
            page: PageType::new_monolith(EsdbPageWidget::default(), None, true),
            status: EsdbStatus::Unknown,
            op_status: OpStatus::Idle,
            project_dir: initial_dir,
            refresh_button: MouseStateHandle::default(),
            export_button: MouseStateHandle::default(),
            import_button: MouseStateHandle::default(),
            backup_button: MouseStateHandle::default(),
            rollback_button: MouseStateHandle::default(),
            compact_button: MouseStateHandle::default(),
            migrate_button: MouseStateHandle::default(),
            replay_button: MouseStateHandle::default(),
        };
        view.fetch_status(ctx);
        view
    }

    fn run_esdb_cmd(
        &mut self,
        label: impl Into<String>,
        cmd_args: &'static [&'static str],
        ctx: &mut ViewContext<Self>,
    ) {
        let label = label.into();
        self.op_status = OpStatus::Running(label);
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
                        .map(|o| {
                            format!(
                                "{}{}",
                                String::from_utf8_lossy(&o.stdout),
                                String::from_utf8_lossy(&o.stderr)
                            )
                        })
                        .map_err(|e| e.to_string())
                };
                let mut py_args = vec!["-m", "specsmith"];
                py_args.extend_from_slice(cmd_args);
                run("py", &py_args)
                    .or_else(|_| run("specsmith", cmd_args))
                    .map_err(|e| format!("specsmith not found: {e}"))
            },
            |me, result: Result<String, String>, ctx| {
                me.op_status = match result {
                    Ok(output) => {
                        // Keep up to 200 lines; the page is now scrollable so long
                        // migration output (e.g. 22+ issue lines) is not clipped.
                        OpStatus::Done(output.lines().take(200).collect::<Vec<_>>().join("\n"))
                    }
                    Err(e) => OpStatus::Error(e.chars().take(200).collect()),
                };
                ctx.notify();
            },
        );
    }

    fn fetch_status(&mut self, ctx: &mut ViewContext<Self>) {
        self.status = EsdbStatus::Loading;
        ctx.notify();
        let project_dir = self.project_dir.clone();
        ctx.spawn(
            async move {
                let run_json = |cmd: &[&str]| -> serde_json::Value {
                    let mut c = std::process::Command::new("py");
                    let mut args = vec!["-m", "specsmith"];
                    args.extend_from_slice(cmd);
                    if let Some(dir) = &project_dir {
                        c.current_dir(dir);
                    }
                    c.args(&args);
                    c.env("SPECSMITH_NO_AUTO_UPDATE", "1");
                    c.env("SPECSMITH_PYPI_CHECKED", "1");
                    c.output()
                        .ok()
                        .and_then(|o| {
                            serde_json::from_str(&String::from_utf8_lossy(&o.stdout)).ok()
                        })
                        .unwrap_or_default()
                };

                let sv = run_json(&["esdb", "status", "--json"]);
                let status_obj = sv.get("status").unwrap_or(&sv);
                let counts_obj = sv.get("counts").unwrap_or(&serde_json::Value::Null);

                let data = EsdbData {
                    available: status_obj
                        .get("available")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    backend: status_obj
                        .get("backend")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_owned(),
                    record_count: status_obj
                        .get("record_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    chain_valid: status_obj
                        .get("chain_valid")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    requirements: counts_obj
                        .get("requirements")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    testcases: counts_obj
                        .get("testcases")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    facts: counts_obj
                        .get("facts")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    hypotheses: counts_obj
                        .get("hypotheses")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    decisions: counts_obj
                        .get("decisions")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    risks: counts_obj
                        .get("risks")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                };
                Ok(data)
            },
            |me, result: Result<EsdbData, String>, ctx| {
                me.status = match result {
                    Ok(data) => EsdbStatus::Loaded(data),
                    Err(e) => EsdbStatus::Error(e),
                };
                ctx.notify();
            },
        );
    }
}

impl Entity for EsdbPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for EsdbPageView {
    type Action = EsdbPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            EsdbPageAction::Refresh => self.fetch_status(ctx),
            EsdbPageAction::Export => {
                self.run_esdb_cmd("esdb export", &["esdb", "export", "--json"], ctx)
            }
            EsdbPageAction::Import => self.run_esdb_cmd(
                "esdb import",
                &["esdb", "import", ".specsmith/esdb_import.json", "--json"],
                ctx,
            ),
            EsdbPageAction::Backup => {
                self.run_esdb_cmd("esdb backup", &["esdb", "backup", "--json"], ctx)
            }
            EsdbPageAction::Rollback => {
                self.run_esdb_cmd("esdb rollback", &["esdb", "rollback", "--json"], ctx)
            }
            EsdbPageAction::Compact => {
                self.run_esdb_cmd("esdb compact", &["esdb", "compact", "--json"], ctx)
            }
            EsdbPageAction::Migrate => self.run_esdb_cmd("esdb migrate", &["esdb", "migrate"], ctx),
            EsdbPageAction::Replay => self.run_esdb_cmd("esdb replay", &["esdb", "replay"], ctx),
        }
    }
}

impl View for EsdbPageView {
    fn ui_name() -> &'static str {
        "EsdbPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

#[derive(Default)]
struct EsdbPageWidget {}

impl EsdbPageWidget {
    fn card(content: Box<dyn Element>, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(content)
            .with_background(appearance.theme().surface_1())
            .with_uniform_padding(16.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_margin_bottom(12.)
            .finish()
    }

    fn stat_row(label: &str, value: &str, appearance: &Appearance) -> Box<dyn Element> {
        Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Expanded::new(
                    1.,
                    Text::new_inline(label.to_string(), appearance.ui_font_family(), 12.)
                        .with_color(appearance.theme().disabled_ui_text_color().into())
                        .finish(),
                )
                .finish(),
            )
            .with_child(
                Text::new_inline(value.to_string(), appearance.monospace_font_family(), 13.)
                    .with_color(appearance.theme().active_ui_text_color().into())
                    .finish(),
            )
            .finish()
    }

    fn op_btn(
        label: impl Into<String>,
        ms: MouseStateHandle,
        action: EsdbPageAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, ms)
            .with_style(UiComponentStyles {
                font_size: Some(11.),
                padding: Some(Coords::uniform(5.)),
                ..Default::default()
            })
            .with_centered_text_label(label.into())
            .build()
            .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
            .finish()
    }
}

impl SettingsWidget for EsdbPageWidget {
    type View = EsdbPageView;

    fn search_terms(&self) -> &str {
        "esdb chronomemory epistemic state database facts hypotheses requirements tests rollback replay backup export import compact migrate"
    }

    fn render(
        &self,
        view: &EsdbPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color();
        let accent: Fill = theme.accent().into_solid().into();

        // ── Section 1: Status ─────────────────────────────────────────────
        let status_header = build_sub_header(appearance, "ChronoMemory ESDB", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let status_card = match &view.status {
            EsdbStatus::Unknown | EsdbStatus::Loading => Self::card(
                Text::new(
                    "Loading ESDB status\u{2026}".to_string(),
                    appearance.ui_font_family(),
                    13.,
                )
                .with_color(dim.into())
                .finish(),
                appearance,
            ),
            EsdbStatus::Error(msg) => Self::card(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(
                        Text::new(
                            "ESDB not available".to_string(),
                            appearance.ui_font_family(),
                            13.,
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new(
                                msg.chars().take(200).collect::<String>(),
                                appearance.monospace_font_family(),
                                10.,
                            )
                            .with_color(theme.ui_error_color().into())
                            .soft_wrap(true)
                            .finish(),
                        )
                        .with_margin_top(6.)
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new(
                                "Start specsmith: specsmith governance-serve".to_string(),
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
            ),
            EsdbStatus::Loaded(data) => {
                let status_color: Fill = if data.available { accent } else { dim.into() };
                let status_text = if data.available {
                    format!("\u{25CF} Online \u{2014} {}", data.backend)
                } else {
                    "\u{25CF} Offline".to_string()
                };
                let chain_icon = if data.chain_valid {
                    "\u{2714}"
                } else {
                    "\u{2717}"
                };
                let chain_color: Fill = if data.chain_valid {
                    accent
                } else {
                    Fill::Solid(theme.ui_error_color())
                };

                {
                    // Compute project DB path for display
                    let db_path = view
                        .project_dir
                        .as_ref()
                        .map(|d| {
                            let p = d.join(".chronomemory");
                            p.to_string_lossy().to_string()
                        })
                        .unwrap_or_else(|| ".chronomemory/".to_string());

                    Self::card(
                        Flex::column()
                            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .with_child(
                                Text::new_inline(status_text, appearance.ui_font_family(), 14.)
                                    .with_color(status_color.into())
                                    .finish(),
                            )
                            .with_child(
                                Container::new(
                                    Flex::row()
                                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                        .with_child(
                                            Text::new_inline(
                                                chain_icon.to_string(),
                                                appearance.ui_font_family(),
                                                12.,
                                            )
                                            .with_color(chain_color.into())
                                            .finish(),
                                        )
                                        .with_child(
                                            Container::new(
                                                Text::new_inline(
                                                    if data.chain_valid {
                                                        " WAL chain integrity OK"
                                                    } else {
                                                        " WAL chain integrity FAILED"
                                                    }
                                                    .to_string(),
                                                    appearance.ui_font_family(),
                                                    12.,
                                                )
                                                .with_color(if data.chain_valid {
                                                    active.into()
                                                } else {
                                                    theme.ui_error_color().into()
                                                })
                                                .finish(),
                                            )
                                            .finish(),
                                        )
                                        .finish(),
                                )
                                .with_margin_top(6.)
                                .finish(),
                            )
                            .with_child(
                                Container::new(
                                    Self::stat_row("Project DB", &db_path, appearance),
                                )
                                .with_margin_top(8.)
                                .finish(),
                            )
                            .with_child(
                                Container::new(
                                    Text::new(
                                        "Run \"Migrate\" to import .specsmith/ JSON → ChronoStore WAL.".to_string(),
                                        appearance.ui_font_family(),
                                        11.,
                                    )
                                    .with_color(dim.into())
                                    .finish(),
                                )
                                .with_margin_top(4.)
                                .finish(),
                            )
                            .finish(),
                        appearance,
                    )
                }
            }
        };

        // ── Section 2: Record counts ──────────────────────────────────────
        let counts_header = build_sub_header(appearance, "Record Counts", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let counts_card = match &view.status {
            EsdbStatus::Loaded(data) => Self::card(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(Self::stat_row(
                        "Total records",
                        &data.record_count.to_string(),
                        appearance,
                    ))
                    .with_child(
                        Container::new(Self::stat_row(
                            "Requirements",
                            &data.requirements.to_string(),
                            appearance,
                        ))
                        .with_margin_top(4.)
                        .finish(),
                    )
                    .with_child(
                        Container::new(Self::stat_row(
                            "Test cases",
                            &data.testcases.to_string(),
                            appearance,
                        ))
                        .with_margin_top(4.)
                        .finish(),
                    )
                    .with_child(
                        Container::new(Self::stat_row(
                            "Facts",
                            &data.facts.to_string(),
                            appearance,
                        ))
                        .with_margin_top(4.)
                        .finish(),
                    )
                    .with_child(
                        Container::new(Self::stat_row(
                            "Hypotheses",
                            &data.hypotheses.to_string(),
                            appearance,
                        ))
                        .with_margin_top(4.)
                        .finish(),
                    )
                    .with_child(
                        Container::new(Self::stat_row(
                            "Decisions",
                            &data.decisions.to_string(),
                            appearance,
                        ))
                        .with_margin_top(4.)
                        .finish(),
                    )
                    .with_child(
                        Container::new(Self::stat_row(
                            "Risks",
                            &data.risks.to_string(),
                            appearance,
                        ))
                        .with_margin_top(4.)
                        .finish(),
                    )
                    .finish(),
                appearance,
            ),
            _ => Self::card(
                Text::new("\u{2014}".to_string(), appearance.ui_font_family(), 13.)
                    .with_color(dim.into())
                    .finish(),
                appearance,
            ),
        };

        // ── Section 3: DB Management ──────────────────────────────────────
        let mgmt_header = build_sub_header(appearance, "Database Management", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let mgmt_desc = Container::new(
            Text::new(
                "Export/import snapshots, create backups, compact the WAL, migrate flat JSON to ESDB, or verify chain integrity via replay.".to_string(),
                appearance.ui_font_family(),
                12.,
            )
            .with_color(dim.into())
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_bottom(10.)
        .finish();

        let row1 = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Self::op_btn(
                "Export",
                view.export_button.clone(),
                EsdbPageAction::Export,
                appearance,
            ))
            .with_child(
                Container::new(Self::op_btn(
                    "Import",
                    view.import_button.clone(),
                    EsdbPageAction::Import,
                    appearance,
                ))
                .with_margin_left(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::op_btn(
                    "Backup",
                    view.backup_button.clone(),
                    EsdbPageAction::Backup,
                    appearance,
                ))
                .with_margin_left(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::op_btn(
                    "Rollback",
                    view.rollback_button.clone(),
                    EsdbPageAction::Rollback,
                    appearance,
                ))
                .with_margin_left(6.)
                .finish(),
            )
            .finish();

        let row2 = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Self::op_btn(
                "Compact",
                view.compact_button.clone(),
                EsdbPageAction::Compact,
                appearance,
            ))
            .with_child(
                Container::new(Self::op_btn(
                    "Migrate",
                    view.migrate_button.clone(),
                    EsdbPageAction::Migrate,
                    appearance,
                ))
                .with_margin_left(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::op_btn(
                    "Replay / verify",
                    view.replay_button.clone(),
                    EsdbPageAction::Replay,
                    appearance,
                ))
                .with_margin_left(6.)
                .finish(),
            )
            .finish();

        let op_output: Option<Box<dyn Element>> = match &view.op_status {
            OpStatus::Idle => None,
            OpStatus::Running(label) => Some(
                Text::new(
                    format!("Running {label}\u{2026}"),
                    appearance.ui_font_family(),
                    11.,
                )
                .with_color(dim.into())
                .finish(),
            ),
            OpStatus::Done(output) => Some(
                Text::new(output.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(active.into())
                    .soft_wrap(true)
                    .finish(),
            ),
            OpStatus::Error(msg) => Some(
                Text::new(msg.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(theme.ui_error_color().into())
                    .soft_wrap(true)
                    .finish(),
            ),
        };

        let mut mgmt_col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(mgmt_desc)
            .with_child(row1)
            .with_child(Container::new(row2).with_margin_top(8.).finish());

        if let Some(out) = op_output {
            mgmt_col.add_child(Container::new(out).with_margin_top(10.).finish());
        }

        let mgmt_card = Self::card(mgmt_col.finish(), appearance);

        // ── Refresh button ────────────────────────────────────────────────
        let refresh_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.refresh_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label("Refresh".to_string())
            .build()
            .on_click(|ctx, _, _| ctx.dispatch_typed_action(EsdbPageAction::Refresh))
            .finish();

        // ── Assemble ──────────────────────────────────────────────────────
        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(status_header)
                    .with_child(status_card)
                    .with_child(render_separator(appearance))
                    .with_child(counts_header)
                    .with_child(counts_card)
                    .with_child(render_separator(appearance))
                    .with_child(mgmt_header)
                    .with_child(mgmt_card)
                    .with_child(Container::new(refresh_btn).with_margin_top(8.).finish())
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

impl SettingsPageMeta for EsdbPageView {
    fn section() -> SettingsSection {
        SettingsSection::Esdb
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn on_page_selected(&mut self, _: bool, ctx: &mut ViewContext<Self>) {
        self.fetch_status(ctx);
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

impl From<ViewHandle<EsdbPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<EsdbPageView>) -> Self {
        SettingsPageViewHandle::Esdb(view_handle)
    }
}
