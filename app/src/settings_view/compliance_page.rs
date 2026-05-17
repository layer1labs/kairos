//! Compliance dashboard — requirement coverage, test coverage, gaps, traceability, governance rules.
//!
//! Sections:
//!  1. Score overview       — overall compliance %, req covered/total, tests linked
//!  2. Governance rules     — H1-H14 hard rules status (specsmith compliance rules)
//!  3. Traceability matrix  — REQ → TEST mapping (specsmith compliance trace)
//!  4. Uncovered / orphans  — gaps + orphaned tests (specsmith compliance gaps)
//!  5. Actions              — Run compliance, Show gaps, Show trace, Check rules

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
// Data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
struct ComplianceScore {
    score_pct: f64,
    total_requirements: usize,
    covered_requirements: usize,
    total_tests: usize,
    linked_tests: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct GovernanceRule {
    id: String,
    name: String,
    /// "ok" | "warning" | "violation"
    status: String,
}

#[derive(Debug, Clone, PartialEq)]
struct TraceEntry {
    requirement_id: String,
    covered: bool,
    tests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct ComplianceData {
    score: ComplianceScore,
    gaps: Vec<String>,
    orphaned_tests: Vec<String>,
    rules: Vec<GovernanceRule>,
    trace: Vec<TraceEntry>,
}

#[derive(Debug, Clone, PartialEq)]
enum ComplianceStatus {
    Unknown,
    Loading,
    Loaded(ComplianceData),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
enum ActionStatus {
    #[default]
    Idle,
    Running(String),
    Done(String),
    Error(String),
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
struct RegulationStatusItem {
    id: String,
    name: String,
    jurisdiction: String,
    status: String, // "compliant" | "partial" | "gap" | "unknown"
    confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
enum RegulationLoadStatus {
    #[default]
    Unknown,
    Loaded(Vec<RegulationStatusItem>),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum CompliancePageAction {
    Refresh,
    RunCompliance,
    ShowGaps,
    ShowTrace,
    CheckRules,
    RunComplianceAudit,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct CompliancePageView {
    page: PageType<Self>,
    status: ComplianceStatus,
    action_status: ActionStatus,
    project_dir: Option<PathBuf>,
    refresh_button: MouseStateHandle,
    run_compliance_button: MouseStateHandle,
    show_gaps_button: MouseStateHandle,
    show_trace_button: MouseStateHandle,
    check_rules_button: MouseStateHandle,
    /// EU/NA regulation compliance status
    regulation_status: RegulationLoadStatus,
    audit_button: MouseStateHandle,
}

impl CompliancePageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let initial_dir = std::env::current_dir().ok();
        let mut view = CompliancePageView {
            page: PageType::new_monolith(CompliancePageWidget::default(), None, true),
            status: ComplianceStatus::Unknown,
            action_status: ActionStatus::Idle,
            project_dir: initial_dir,
            refresh_button: MouseStateHandle::default(),
            run_compliance_button: MouseStateHandle::default(),
            show_gaps_button: MouseStateHandle::default(),
            show_trace_button: MouseStateHandle::default(),
            check_rules_button: MouseStateHandle::default(),
            regulation_status: RegulationLoadStatus::Unknown,
            audit_button: MouseStateHandle::default(),
        };
        view.fetch_all(ctx);
        view.fetch_regulation_status(ctx);
        view
    }

    fn run_cmd(
        &mut self,
        label: impl Into<String>,
        cmd_args: &'static [&'static str],
        ctx: &mut ViewContext<Self>,
    ) {
        let label = label.into();
        self.action_status = ActionStatus::Running(label.clone());
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
                me.action_status = match result {
                    Ok(output) => {
                        ActionStatus::Done(output.lines().take(30).collect::<Vec<_>>().join("\n"))
                    }
                    Err(e) => ActionStatus::Error(e.chars().take(200).collect()),
                };
                ctx.notify();
            },
        );
    }

    fn fetch_regulation_status(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.spawn(
            async move {
                // Call GET http://127.0.0.1:7700/api/compliance/status
                let url = "http://127.0.0.1:7700/api/compliance/status";
                let out = std::process::Command::new("curl")
                    .args(["-s", "--max-time", "10", url])
                    .output()
                    .map_err(|e| e.to_string())?;
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                Ok(text)
            },
            |me, result: Result<String, String>, ctx| {
                me.regulation_status = match result {
                    Ok(text) => {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                            let regs = v
                                .get("regulations")
                                .and_then(|r| r.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|r| {
                                            Some(RegulationStatusItem {
                                                id: r.get("regulation_id")?.as_str()?.to_owned(),
                                                name: r
                                                    .get("regulation_name")?
                                                    .as_str()?
                                                    .to_owned(),
                                                jurisdiction: r
                                                    .get("jurisdiction")
                                                    .and_then(|j| j.as_str())
                                                    .unwrap_or("")
                                                    .to_owned(),
                                                status: r
                                                    .get("overall_status")
                                                    .and_then(|s| s.as_str())
                                                    .unwrap_or("unknown")
                                                    .to_owned(),
                                                confidence: r
                                                    .get("overall_confidence")
                                                    .and_then(|c| c.as_f64())
                                                    .unwrap_or(0.0),
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            RegulationLoadStatus::Loaded(regs)
                        } else {
                            RegulationLoadStatus::Unknown
                        }
                    }
                    Err(_) => RegulationLoadStatus::Unknown,
                };
                ctx.notify();
            },
        );
    }

    fn run_compliance_audit(&mut self, ctx: &mut ViewContext<Self>) {
        self.run_cmd("compliance audit", &["compliance", "audit", "--json"], ctx);
    }

    fn fetch_all(&mut self, ctx: &mut ViewContext<Self>) {
        self.status = ComplianceStatus::Loading;
        ctx.notify();
        let project_dir = self.project_dir.clone();
        ctx.spawn(
            async move {
                let run_json = |cmd: &[&str]| -> Result<serde_json::Value, String> {
                    let mut c = std::process::Command::new("py");
                    let mut args = vec!["-m", "specsmith"];
                    args.extend_from_slice(cmd);
                    if let Some(dir) = &project_dir {
                        c.current_dir(dir);
                    }
                    c.args(&args);
                    c.env("SPECSMITH_NO_AUTO_UPDATE", "1");
                    c.env("SPECSMITH_PYPI_CHECKED", "1");
                    let out = c.output().map_err(|e| e.to_string())?;
                    let text = String::from_utf8_lossy(&out.stdout).to_string();
                    serde_json::from_str(&text).map_err(|e| e.to_string())
                };
                let run_text = |cmd: &[&str]| -> String {
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
                        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                        .unwrap_or_default()
                };

                // Traceability matrix — `specsmith req trace`
                // Output: "  \u2713 REQ-001              \u2192 TEST-001, TEST-002"
                //     or: "  \u2717 REQ-004              (no tests)"
                let trace_text = run_text(&["req", "trace"]);
                let trace: Vec<TraceEntry> = trace_text
                    .lines()
                    .filter_map(|line| {
                        let line = line.trim();
                        if line.is_empty() {
                            return None;
                        }
                        let covered = line.contains('\u{2713}') || line.starts_with('\u{2713}');
                        // strip leading ✓/✗ and whitespace
                        let rest = line
                            .trim_start_matches(['\u{2713}', '\u{2717}', ' '])
                            .trim();
                        if rest.is_empty() {
                            return None;
                        }
                        if let Some((req, tests_part)) = rest.split_once('\u{2192}') {
                            let req_id = req.trim().to_owned();
                            if req_id.is_empty() {
                                return None;
                            }
                            let tests: Vec<String> = tests_part
                                .split(',')
                                .map(|s| s.trim().to_owned())
                                .filter(|s| !s.is_empty())
                                .collect();
                            Some(TraceEntry {
                                requirement_id: req_id,
                                covered: !tests.is_empty(),
                                tests,
                            })
                        } else {
                            // "REQ-XXX (no tests)" or "REQ-XXX"
                            let req_id = rest.split_whitespace().next().unwrap_or("?").to_owned();
                            Some(TraceEntry {
                                requirement_id: req_id,
                                covered,
                                tests: vec![],
                            })
                        }
                    })
                    .collect();

                // Compute coverage counts from trace
                let total_requirements = trace.len();
                let covered_requirements = trace.iter().filter(|e| e.covered).count();
                let linked_tests: usize = trace.iter().map(|e| e.tests.len()).sum();

                // Gaps — `specsmith req gaps`
                let gaps_text = run_text(&["req", "gaps"]);
                let gaps: Vec<String> =
                    if gaps_text.to_lowercase().contains("all requirements have") {
                        vec![]
                    } else {
                        gaps_text
                            .lines()
                            .map(|l| l.trim().to_owned())
                            .filter(|l| !l.is_empty())
                            .collect()
                    };

                // Orphaned tests — `specsmith req orphans`
                let orphans_text = run_text(&["req", "orphans"]);
                let orphaned_tests: Vec<String> =
                    if orphans_text.to_lowercase().contains("no orphaned") {
                        vec![]
                    } else {
                        orphans_text
                            .lines()
                            .map(|l| l.trim().to_owned())
                            .filter(|l| !l.is_empty())
                            .collect()
                    };

                // Score: req coverage ratio (primary); validate ratio (fallback)
                let vv = run_json(&["validate", "--strict", "--json"]).unwrap_or_default();
                let std_passed =
                    vv.get("std_passed").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let std_failed =
                    vv.get("std_failed").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let total_checks = std_passed + std_failed;
                let score_pct = if total_requirements > 0 {
                    (covered_requirements as f64 / total_requirements as f64) * 100.0
                } else if total_checks > 0 {
                    (std_passed as f64 / total_checks as f64) * 100.0
                } else {
                    0.0
                };
                let score = ComplianceScore {
                    score_pct,
                    total_requirements,
                    covered_requirements,
                    total_tests: linked_tests,
                    linked_tests,
                };

                // Rules — `specsmith rules list`
                let rules_text = run_text(&["rules", "list"]);
                let rules: Vec<GovernanceRule> = rules_text
                    .lines()
                    .filter_map(|line| {
                        let line = line.trim();
                        if line.is_empty() {
                            return None;
                        }
                        let status = if line.contains('\u{2713}') || line.starts_with("ok") {
                            "ok"
                        } else if line.contains('\u{26A0}')
                            || line.to_lowercase().contains("warning")
                        {
                            "warning"
                        } else {
                            "violation"
                        };
                        let rest = line.trim_start_matches(|c: char| !c.is_alphanumeric());
                        let (id, name) = rest.split_once(':').unwrap_or(("?", rest));
                        Some(GovernanceRule {
                            id: id.trim().to_owned(),
                            name: name.trim().chars().take(80).collect(),
                            status: status.to_owned(),
                        })
                    })
                    .collect();

                Ok(ComplianceData {
                    score,
                    gaps,
                    orphaned_tests,
                    rules,
                    trace,
                })
            },
            |me, result: Result<ComplianceData, String>, ctx| {
                me.status = match result {
                    Ok(data) => ComplianceStatus::Loaded(data),
                    Err(e) => ComplianceStatus::Error(e),
                };
                ctx.notify();
            },
        );
    }
}

impl Entity for CompliancePageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for CompliancePageView {
    type Action = CompliancePageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            CompliancePageAction::Refresh => self.fetch_all(ctx),
            CompliancePageAction::RunCompliance => {
                // `specsmith validate` — governance consistency check
                self.run_cmd("validate", &["validate", "--strict"], ctx)
            }
            CompliancePageAction::ShowGaps => {
                // `specsmith req gaps` — list requirements without test coverage
                self.run_cmd("req gaps", &["req", "gaps"], ctx)
            }
            CompliancePageAction::ShowTrace => {
                // `specsmith req trace` — REQ → TEST traceability matrix
                self.run_cmd("req trace", &["req", "trace"], ctx)
            }
            CompliancePageAction::CheckRules => {
                // `specsmith rules list` — governance rule documents
                self.run_cmd("rules list", &["rules", "list"], ctx)
            }
            CompliancePageAction::RunComplianceAudit => {
                self.run_compliance_audit(ctx);
                self.fetch_regulation_status(ctx);
            }
        }
    }
}

impl View for CompliancePageView {
    fn ui_name() -> &'static str {
        "CompliancePage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CompliancePageWidget {}

impl CompliancePageWidget {
    fn card(content: Box<dyn Element>, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(content)
            .with_background(appearance.theme().surface_1())
            .with_uniform_padding(16.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_margin_bottom(12.)
            .finish()
    }

    fn stat_row(
        label: &str,
        value: &str,
        color: Fill,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
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
                    .with_color(color.into())
                    .finish(),
            )
            .finish()
    }

    fn small_btn(
        label: impl Into<String>,
        ms: MouseStateHandle,
        action: CompliancePageAction,
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

impl SettingsWidget for CompliancePageWidget {
    type View = CompliancePageView;

    fn search_terms(&self) -> &str {
        "compliance requirements coverage tests gaps traceability governance rules H1 H22 orphaned REQ TEST"
    }

    fn render(
        &self,
        view: &CompliancePageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color();
        let accent: Fill = theme.accent().into_solid().into();
        let warn: Fill = Fill::Solid(theme.ui_error_color());

        // Loading / error
        if let ComplianceStatus::Unknown | ComplianceStatus::Loading = &view.status {
            return Container::new(
                Text::new(
                    "Loading compliance data\u{2026}".to_string(),
                    appearance.ui_font_family(),
                    13.,
                )
                .with_color(dim.into())
                .finish(),
            )
            .with_uniform_padding(28.)
            .finish();
        }
        if let ComplianceStatus::Error(msg) = &view.status {
            return Container::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(
                        Text::new(
                            "Compliance data unavailable".to_string(),
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
                    .finish(),
            )
            .with_uniform_padding(28.)
            .finish();
        }
        let data = match &view.status {
            ComplianceStatus::Loaded(d) => d,
            _ => unreachable!(),
        };

        // ── Section 1: Score ──────────────────────────────────────────────
        let score_header = build_sub_header(appearance, "Compliance Score", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let sp = data.score.score_pct;
        let score_fill: Fill = if sp >= 80.0 {
            accent
        } else if sp >= 50.0 {
            active.into()
        } else {
            warn
        };
        let score_card = Self::card(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(
                    Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            Text::new_inline("\u{25CF}", appearance.ui_font_family(), 18.)
                                .with_color(score_fill.into())
                                .finish(),
                        )
                        .with_child(
                            Container::new(
                                Text::new_inline(
                                    format!("  Overall: {:.0}%", sp),
                                    appearance.ui_font_family(),
                                    16.,
                                )
                                .with_color(active.into())
                                .finish(),
                            )
                            .finish(),
                        )
                        .finish(),
                )
                .with_child(
                    Container::new(Self::stat_row(
                        "Requirements covered",
                        &format!(
                            "{}/{}",
                            data.score.covered_requirements, data.score.total_requirements
                        ),
                        if data.score.covered_requirements == data.score.total_requirements {
                            accent
                        } else {
                            active.into()
                        },
                        appearance,
                    ))
                    .with_margin_top(12.)
                    .finish(),
                )
                .with_child(
                    Container::new(Self::stat_row(
                        "Tests linked",
                        &format!("{}/{}", data.score.linked_tests, data.score.total_tests),
                        active.into(),
                        appearance,
                    ))
                    .with_margin_top(6.)
                    .finish(),
                )
                .with_child(
                    Container::new(Self::stat_row(
                        "Uncovered requirements",
                        &data.gaps.len().to_string(),
                        if data.gaps.is_empty() { accent } else { warn },
                        appearance,
                    ))
                    .with_margin_top(6.)
                    .finish(),
                )
                .with_child(
                    Container::new(Self::stat_row(
                        "Orphaned tests",
                        &data.orphaned_tests.len().to_string(),
                        if data.orphaned_tests.is_empty() {
                            accent
                        } else {
                            warn
                        },
                        appearance,
                    ))
                    .with_margin_top(6.)
                    .finish(),
                )
                .finish(),
            appearance,
        );

        // ── Section 2: Governance rules H1-H14 ───────────────────────────
        let rules_header =
            build_sub_header(appearance, "Governance Hard Rules (H1\u{2013}H22)", None)
                .with_padding_bottom(HEADER_PADDING)
                .finish();
        let rules_card = if data.rules.is_empty() {
            Self::card(
                Text::new(
                    "Click \u{201c}Rules\u{201d} to fetch hard-rule status.".to_string(),
                    appearance.ui_font_family(),
                    12.,
                )
                .with_color(dim.into())
                .finish(),
                appearance,
            )
        } else {
            let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
            for (i, rule) in data.rules.iter().take(30).enumerate() {
                let (icon, color) = match rule.status.as_str() {
                    "ok" => ("\u{2713}", accent),
                    "warning" => ("\u{26A0}", active.into()),
                    _ => ("\u{2717}", warn),
                };
                let row = Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(
                        Container::new(
                            Text::new_inline(icon.to_string(), appearance.ui_font_family(), 12.)
                                .with_color(color.into())
                                .finish(),
                        )
                        .with_margin_right(8.)
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new_inline(
                                rule.id.clone(),
                                appearance.monospace_font_family(),
                                11.,
                            )
                            .with_color(dim.into())
                            .finish(),
                        )
                        .with_margin_right(6.)
                        .finish(),
                    )
                    .with_child(
                        Text::new_inline(
                            rule.name.chars().take(70).collect::<String>(),
                            appearance.ui_font_family(),
                            12.,
                        )
                        .with_color(active.into())
                        .finish(),
                    )
                    .finish();
                if i > 0 {
                    col.add_child(Container::new(row).with_margin_top(5.).finish());
                } else {
                    col.add_child(row);
                }
            }
            Self::card(col.finish(), appearance)
        };

        // ── Section 3: Traceability matrix ────────────────────────────────
        let trace_header = build_sub_header(appearance, "REQ \u{2192} TEST Traceability", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();
        let trace_card = if data.trace.is_empty() {
            Self::card(
                Text::new(
                    "Click \u{201c}Trace\u{201d} to load the traceability matrix.".to_string(),
                    appearance.ui_font_family(),
                    12.,
                )
                .with_color(dim.into())
                .finish(),
                appearance,
            )
        } else {
            let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
            for (i, entry) in data.trace.iter().take(30).enumerate() {
                let icon = if entry.covered {
                    "\u{2713}"
                } else {
                    "\u{2717}"
                };
                let color = if entry.covered { accent } else { warn };
                let tests_str = if entry.tests.is_empty() {
                    "(none)".to_string()
                } else {
                    entry.tests.join(", ")
                };
                let row = Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(
                        Container::new(
                            Text::new_inline(
                                icon.to_string(),
                                appearance.monospace_font_family(),
                                11.,
                            )
                            .with_color(color.into())
                            .finish(),
                        )
                        .with_margin_right(6.)
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new_inline(
                                entry.requirement_id.clone(),
                                appearance.monospace_font_family(),
                                11.,
                            )
                            .with_color(dim.into())
                            .finish(),
                        )
                        .with_margin_right(8.)
                        .finish(),
                    )
                    .with_child(
                        Text::new_inline(
                            format!("\u{2192}  {tests_str}"),
                            appearance.monospace_font_family(),
                            10.,
                        )
                        .with_color(active.into())
                        .finish(),
                    )
                    .finish();
                if i > 0 {
                    col.add_child(Container::new(row).with_margin_top(4.).finish());
                } else {
                    col.add_child(row);
                }
            }
            if data.trace.len() > 30 {
                col.add_child(
                    Container::new(
                        Text::new(
                            format!("\u{2026} and {} more", data.trace.len() - 30),
                            appearance.ui_font_family(),
                            11.,
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .with_margin_top(4.)
                    .finish(),
                );
            }
            Self::card(col.finish(), appearance)
        };

        // ── Section 4: Gaps + orphans ─────────────────────────────────────
        let gaps_header = build_sub_header(appearance, "Uncovered Requirements", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();
        let gaps_card = if data.gaps.is_empty() && data.orphaned_tests.is_empty() {
            Self::card(
                Text::new(
                    "\u{2714}  All requirements covered. No orphaned tests.".to_string(),
                    appearance.ui_font_family(),
                    13.,
                )
                .with_color(accent.into())
                .finish(),
                appearance,
            )
        } else {
            let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
            if !data.gaps.is_empty() {
                col.add_child(
                    Text::new(
                        format!("Uncovered ({}):", data.gaps.len()),
                        appearance.ui_font_family(),
                        12.,
                    )
                    .with_color(dim.into())
                    .finish(),
                );
                for gap in data.gaps.iter().take(15) {
                    col.add_child(
                        Container::new(
                            Text::new(
                                format!("  \u{2717}  {gap}"),
                                appearance.monospace_font_family(),
                                11.,
                            )
                            .with_color(warn.into())
                            .soft_wrap(true)
                            .finish(),
                        )
                        .with_margin_top(3.)
                        .finish(),
                    );
                }
            }
            if !data.orphaned_tests.is_empty() {
                col.add_child(
                    Container::new(
                        Text::new(
                            format!("Orphaned tests ({}):", data.orphaned_tests.len()),
                            appearance.ui_font_family(),
                            12.,
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .with_margin_top(10.)
                    .finish(),
                );
                for t in data.orphaned_tests.iter().take(10) {
                    col.add_child(
                        Container::new(
                            Text::new(
                                format!("  \u{26A0}  {t}"),
                                appearance.monospace_font_family(),
                                11.,
                            )
                            .with_color(active.into())
                            .soft_wrap(true)
                            .finish(),
                        )
                        .with_margin_top(3.)
                        .finish(),
                    );
                }
            }
            Self::card(col.finish(), appearance)
        };

        // ── Action output ─────────────────────────────────────────────────
        let action_output: Option<Box<dyn Element>> = match &view.action_status {
            ActionStatus::Idle => None,
            ActionStatus::Running(label) => Some(
                Text::new(
                    format!("Running {label}\u{2026}"),
                    appearance.ui_font_family(),
                    11.,
                )
                .with_color(dim.into())
                .finish(),
            ),
            ActionStatus::Done(output) => Some(
                Text::new(output.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(active.into())
                    .soft_wrap(true)
                    .finish(),
            ),
            ActionStatus::Error(msg) => Some(
                Text::new(msg.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(theme.ui_error_color().into())
                    .soft_wrap(true)
                    .finish(),
            ),
        };

        // ── Section 5: EU/NA Regulation Compliance ────────────────────────
        let reg_header = build_sub_header(appearance, "EU / NA Regulation Compliance", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let reg_card = match &view.regulation_status {
            RegulationLoadStatus::Unknown => Self::card(
                Text::new(
                    "Click \u{201c}Regulation Audit\u{201d} to check EU/NA AI regulation compliance.".to_string(),
                    appearance.ui_font_family(),
                    12.,
                )
                .with_color(dim.into())
                .finish(),
                appearance,
            ),
            RegulationLoadStatus::Error(e) => Self::card(
                Text::new(
                    format!("Error: {}", &e[..e.len().min(100)]),
                    appearance.ui_font_family(),
                    11.,
                )
                .with_color(theme.ui_error_color().into())
                .finish(),
                appearance,
            ),
            RegulationLoadStatus::Loaded(regs) => {
                let mut col =
                    Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
                for (i, reg) in regs.iter().enumerate() {
                    let (status_color, status_icon) = match reg.status.as_str() {
                        "compliant" => (accent, "\u{2714}"),
                        "partial" => (active.into(), "\u{26A0}"),
                        "gap" => (warn, "\u{2717}"),
                        _ => (dim.into(), "\u{2014}"),
                    };
                    let row = Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            Container::new(
                                Text::new_inline(
                                    status_icon.to_string(),
                                    appearance.ui_font_family(),
                                    12.,
                                )
                                .with_color(status_color.into())
                                .finish(),
                            )
                            .with_margin_right(8.)
                            .finish(),
                        )
                        .with_child(
                            Expanded::new(
                                1.,
                                Text::new_inline(
                                    reg.name.chars().take(40).collect::<String>(),
                                    appearance.ui_font_family(),
                                    12.,
                                )
                                .with_color(active.into())
                                .finish(),
                            )
                            .finish(),
                        )
                        .with_child(
                            Text::new_inline(
                                format!(
                                    "{}  {:.0}%",
                                    reg.jurisdiction,
                                    reg.confidence * 100.0
                                ),
                                appearance.monospace_font_family(),
                                10.,
                            )
                            .with_color(dim.into())
                            .finish(),
                        )
                        .finish();
                    if i > 0 {
                        col.add_child(Container::new(row).with_margin_top(6.).finish());
                    } else {
                        col.add_child(row);
                    }
                }
                Self::card(col.finish(), appearance)
            }
        };

        // Regulation audit button
        let audit_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.audit_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(11.),
                padding: Some(Coords::uniform(5.)),
                ..Default::default()
            })
            .with_centered_text_label("Regulation Audit".to_string())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(CompliancePageAction::RunComplianceAudit)
            })
            .finish();

        // ── Actions bar ───────────────────────────────────────────────────
        let actions_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Self::small_btn(
                "Refresh",
                view.refresh_button.clone(),
                CompliancePageAction::Refresh,
                appearance,
            ))
            .with_child(
                Container::new(Self::small_btn(
                    "Run compliance",
                    view.run_compliance_button.clone(),
                    CompliancePageAction::RunCompliance,
                    appearance,
                ))
                .with_margin_left(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::small_btn(
                    "Gaps",
                    view.show_gaps_button.clone(),
                    CompliancePageAction::ShowGaps,
                    appearance,
                ))
                .with_margin_left(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::small_btn(
                    "Trace",
                    view.show_trace_button.clone(),
                    CompliancePageAction::ShowTrace,
                    appearance,
                ))
                .with_margin_left(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::small_btn(
                    "Rules",
                    view.check_rules_button.clone(),
                    CompliancePageAction::CheckRules,
                    appearance,
                ))
                .with_margin_left(6.)
                .finish(),
            )
            .finish();

        let mut output_col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        if let Some(elem) = action_output {
            output_col.add_child(Container::new(elem).with_margin_top(8.).finish());
        }

        // ── Assemble ──────────────────────────────────────────────────────
        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(score_header)
                    .with_child(score_card)
                    .with_child(render_separator(appearance))
                    .with_child(rules_header)
                    .with_child(rules_card)
                    .with_child(render_separator(appearance))
                    .with_child(trace_header)
                    .with_child(trace_card)
                    .with_child(render_separator(appearance))
                    .with_child(gaps_header)
                    .with_child(gaps_card)
                    .with_child(render_separator(appearance))
                    .with_child(reg_header)
                    .with_child(reg_card)
                    .with_child(
                        Container::new(audit_btn)
                            .with_margin_top(6.)
                            .with_margin_bottom(4.)
                            .finish(),
                    )
                    .with_child(render_separator(appearance))
                    .with_child(Container::new(actions_row).with_margin_top(4.).finish())
                    .with_child(output_col.finish())
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

impl SettingsPageMeta for CompliancePageView {
    fn section() -> SettingsSection {
        SettingsSection::Compliance
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn on_page_selected(&mut self, _: bool, ctx: &mut ViewContext<Self>) {
        self.fetch_all(ctx);
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

impl From<ViewHandle<CompliancePageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<CompliancePageView>) -> Self {
        SettingsPageViewHandle::Compliance(view_handle)
    }
}
