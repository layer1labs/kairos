//! Token Usage settings page (REQ-020).
//!
//! Displays a live summary fetched from `specsmith credits summary --json`:
//!  - total tokens in / out, total cost USD
//!  - session / entry counts
//!  - per-model breakdown
//!  - budget status (if configured)
//!  - "Refresh" button — re-runs the subprocess
//!  - hint for clearing history via `specsmith credits record --clear`

use super::{
    settings_page::{
        build_sub_header, MatchData, PageType, SettingsPageEvent, SettingsPageMeta,
        SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use warpui::{
    elements::{
        ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element, Flex,
        MouseStateHandle, ParentElement, Radius, Text,
    },
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponentStyles},
    },
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ModelRow {
    pub model: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Default)]
pub struct BudgetInfo {
    pub limit_usd: f64,
    pub spent_usd: f64,
}

#[derive(Debug, Clone, Default)]
pub struct CreditsSummary {
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_cost_usd: f64,
    pub session_count: u32,
    pub entry_count: u32,
    pub by_model: Vec<ModelRow>,
    pub alerts: Vec<String>,
    pub budget: Option<BudgetInfo>,
}

// ---------------------------------------------------------------------------
// Load status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub enum LoadStatus {
    #[default]
    Idle,
    Loading,
    Loaded(CreditsSummary),
    Error(String),
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum TokenUsagePageAction {
    Refresh,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct TokenUsagePageView {
    page: PageType<Self>,
    pub(crate) status: LoadStatus,
    pub(crate) refresh_button: MouseStateHandle,
}

impl TokenUsagePageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let mut view = Self {
            page: PageType::new_monolith(TokenUsageWidget::default(), None, false),
            status: LoadStatus::Idle,
            refresh_button: MouseStateHandle::default(),
        };
        view.load(ctx);
        view
    }

    fn load(&mut self, ctx: &mut ViewContext<Self>) {
        self.status = LoadStatus::Loading;
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
                let home = dirs::home_dir()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|| "~".to_string());
                let result = run(
                    "py",
                    &[
                        "-m",
                        "specsmith",
                        "credits",
                        "summary",
                        "--json",
                        "--project-dir",
                        &home,
                    ],
                )
                .or_else(|_| {
                    run(
                        "specsmith",
                        &["credits", "summary", "--json", "--project-dir", &home],
                    )
                });
                match result {
                    Ok(out) if out.status.success() => {
                        Ok(String::from_utf8_lossy(&out.stdout).to_string())
                    }
                    Ok(out) => {
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        Err(format!(
                            "specsmith exited with status {}: {}",
                            out.status, stderr
                        ))
                    }
                    Err(e) => Err(format!("specsmith not found: {e}")),
                }
            },
            |me, result: Result<String, String>, ctx| {
                me.status = match result {
                    Ok(json) => match Self::parse_summary(&json) {
                        Ok(summary) => LoadStatus::Loaded(summary),
                        Err(e) => LoadStatus::Error(format!("Parse error: {e}")),
                    },
                    Err(e) => LoadStatus::Error(e),
                };
                ctx.notify();
            },
        );
    }

    fn parse_summary(json: &str) -> Result<CreditsSummary, String> {
        let val: serde_json::Value =
            serde_json::from_str(json).map_err(|e| e.to_string())?;

        let get_u64 = |v: &serde_json::Value, key: &str| -> u64 {
            v.get(key).and_then(|x| x.as_u64()).unwrap_or(0)
        };
        let get_u32 =
            |v: &serde_json::Value, key: &str| -> u32 { get_u64(v, key) as u32 };
        let get_f64 = |v: &serde_json::Value, key: &str| -> f64 {
            v.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0)
        };

        let total_tokens_in = get_u64(&val, "total_tokens_in");
        let total_tokens_out = get_u64(&val, "total_tokens_out");
        let total_cost_usd = get_f64(&val, "total_cost_usd");
        let session_count = get_u32(&val, "session_count");
        let entry_count = get_u32(&val, "entry_count");

        let alerts: Vec<String> = val
            .get("alerts")
            .and_then(|a| a.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let by_model: Vec<ModelRow> = val
            .get("by_model")
            .and_then(|bm| bm.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(model, mval)| ModelRow {
                        model: model.clone(),
                        tokens_in: get_u64(mval, "tokens_in"),
                        tokens_out: get_u64(mval, "tokens_out"),
                        cost_usd: get_f64(mval, "cost_usd"),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let budget = val.get("budget").and_then(|b| {
            if b.is_null() {
                return None;
            }
            Some(BudgetInfo {
                limit_usd: get_f64(b, "limit_usd"),
                spent_usd: get_f64(b, "spent_usd"),
            })
        });

        Ok(CreditsSummary {
            total_tokens_in,
            total_tokens_out,
            total_cost_usd,
            session_count,
            entry_count,
            by_model,
            alerts,
            budget,
        })
    }
}

// ---------------------------------------------------------------------------
// Entity + TypedActionView + View
// ---------------------------------------------------------------------------

impl Entity for TokenUsagePageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for TokenUsagePageView {
    type Action = TokenUsagePageAction;

    fn handle_action(&mut self, action: &TokenUsagePageAction, ctx: &mut ViewContext<Self>) {
        match action {
            TokenUsagePageAction::Refresh => {
                self.load(ctx);
            }
        }
    }
}

impl View for TokenUsagePageView {
    fn ui_name() -> &'static str {
        "TokenUsagePage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

// ---------------------------------------------------------------------------
// SettingsPageMeta
// ---------------------------------------------------------------------------

impl SettingsPageMeta for TokenUsagePageView {
    fn section() -> SettingsSection {
        SettingsSection::TokenUsage
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, _query: &str, _ctx: &mut ViewContext<Self>) -> MatchData {
        MatchData::Uncounted(true)
    }

    fn scroll_to_widget(&mut self, _widget_id: &'static str) {}
    fn clear_highlighted_widget(&mut self) {}
}

impl From<ViewHandle<TokenUsagePageView>> for SettingsPageViewHandle {
    fn from(h: ViewHandle<TokenUsagePageView>) -> Self {
        SettingsPageViewHandle::TokenUsage(h)
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

#[derive(Default)]
struct TokenUsageWidget {}

impl TokenUsageWidget {
    fn stat_row(
        label: &str,
        value: impl Into<String>,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let dim = appearance.theme().disabled_ui_text_color();
        let active = appearance.theme().active_ui_text_color();
        Container::new(
            Flex::row()
                .with_child(
                    ConstrainedBox::new(
                        Text::new_inline(label, appearance.ui_font_family(), 12.)
                            .with_color(dim.into())
                            .finish(),
                    )
                    .with_width(160.)
                    .finish(),
                )
                .with_child(
                    Text::new_inline(value.into(), appearance.ui_font_family(), 12.)
                        .with_color(active.into())
                        .finish(),
                )
                .finish(),
        )
        .with_margin_bottom(4.)
        .finish()
    }

    fn card(content: Box<dyn Element>, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(content)
            .with_background(appearance.theme().surface_1())
            .with_uniform_padding(14.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_margin_bottom(12.)
            .finish()
    }
}

fn format_num(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

impl SettingsWidget for TokenUsageWidget {
    type View = TokenUsagePageView;

    fn search_terms(&self) -> &str {
        "token usage credits cost budget model specsmith billing"
    }

    fn render(
        &self,
        view: &TokenUsagePageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let dim = appearance.theme().disabled_ui_text_color();
        let active = appearance.theme().active_ui_text_color();

        let mut col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch);

        // ── Title ──────────────────────────────────────────────────────────
        col = col.with_child(
            build_sub_header(appearance, "Token Usage", None)
                .with_padding_bottom(HEADER_PADDING)
                .finish(),
        );

        match &view.status {
            LoadStatus::Idle | LoadStatus::Loading => {
                col = col.with_child(
                    Container::new(
                        Text::new_inline("Loading\u{2026}", appearance.ui_font_family(), 13.)
                            .with_color(dim.into())
                            .finish(),
                    )
                    .with_margin_bottom(8.)
                    .finish(),
                );
            }

            LoadStatus::Error(msg) => {
                let msg = msg.clone();
                col = col.with_child(Self::card(
                    Text::new(
                        format!("Could not load usage data: {msg}"),
                        appearance.ui_font_family(),
                        12.,
                    )
                    .with_color(dim.into())
                    .soft_wrap(true)
                    .finish(),
                    appearance,
                ));
            }

            LoadStatus::Loaded(summary) => {
                let summary = summary.clone();

                // ── Alerts ────────────────────────────────────────────────
                for alert in &summary.alerts {
                    let alert = alert.clone();
                    col = col.with_child(
                        Container::new(
                            Text::new_inline(
                                format!("\u{26A0}  {alert}"),
                                appearance.ui_font_family(),
                                12.,
                            )
                            .with_color(active.into())
                            .finish(),
                        )
                        .with_margin_bottom(6.)
                        .finish(),
                    );
                }

                // ── Budget ────────────────────────────────────────────────
                if let Some(ref budget) = summary.budget {
                    let pct = if budget.limit_usd > 0.0 {
                        budget.spent_usd / budget.limit_usd * 100.0
                    } else {
                        0.0
                    };
                    col = col.with_child(
                        build_sub_header(appearance, "Budget", None)
                            .with_padding_bottom(4.)
                            .finish(),
                    );
                    col = col.with_child(Self::card(
                        Text::new_inline(
                            format!(
                                "${:.4} / ${:.4}  ({:.1}%)",
                                budget.spent_usd, budget.limit_usd, pct
                            ),
                            appearance.ui_font_family(),
                            12.,
                        )
                        .with_color(active.into())
                        .finish(),
                        appearance,
                    ));
                }

                // ── Overall stats ─────────────────────────────────────────
                col = col.with_child(
                    build_sub_header(appearance, "Overall Usage", None)
                        .with_padding_bottom(8.)
                        .finish(),
                );
                col = col.with_child(Self::card(
                    Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .with_child(Self::stat_row(
                            "Tokens in",
                            format_num(summary.total_tokens_in),
                            appearance,
                        ))
                        .with_child(Self::stat_row(
                            "Tokens out",
                            format_num(summary.total_tokens_out),
                            appearance,
                        ))
                        .with_child(Self::stat_row(
                            "Total cost",
                            format!("${:.6}", summary.total_cost_usd),
                            appearance,
                        ))
                        .with_child(Self::stat_row(
                            "Sessions",
                            summary.session_count.to_string(),
                            appearance,
                        ))
                        .with_child(Self::stat_row(
                            "Log entries",
                            summary.entry_count.to_string(),
                            appearance,
                        ))
                        .finish(),
                    appearance,
                ));

                // ── Per-model breakdown ───────────────────────────────────
                if !summary.by_model.is_empty() {
                    col = col.with_child(
                        build_sub_header(appearance, "By Model", None)
                            .with_padding_bottom(8.)
                            .finish(),
                    );
                    let mut models = summary.by_model.clone();
                    models.sort_by(|a, b| {
                        b.cost_usd
                            .partial_cmp(&a.cost_usd)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let mut model_col = Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Stretch);
                    for row in &models {
                        let line = format!(
                            "{}    in: {}  out: {}  ${:.6}",
                            row.model,
                            format_num(row.tokens_in),
                            format_num(row.tokens_out),
                            row.cost_usd,
                        );
                        model_col = model_col.with_child(
                            Container::new(
                                Text::new(line, appearance.monospace_font_family(), 11.)
                                    .with_color(dim.into())
                                    .finish(),
                            )
                            .with_margin_bottom(2.)
                            .finish(),
                        );
                    }
                    col = col.with_child(Self::card(model_col.finish(), appearance));
                }
            }
        }

        // ── Refresh button ─────────────────────────────────────────────────
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
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(TokenUsagePageAction::Refresh);
            })
            .finish();

        col = col.with_child(Container::new(refresh_btn).with_margin_bottom(8.).finish());

        // ── Clear hint ─────────────────────────────────────────────────────
        col = col.with_child(
            Container::new(
                Text::new(
                    "To clear usage history: specsmith credits record --clear".to_string(),
                    appearance.ui_font_family(),
                    11.,
                )
                .with_color(dim.into())
                .finish(),
            )
            .with_margin_bottom(HEADER_PADDING)
            .finish(),
        );

        col.finish()
    }
}
