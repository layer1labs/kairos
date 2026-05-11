//! Compliance dashboard page — requirement coverage, test coverage, gaps, traceability.
//!
//! Sections:
//!  1. Compliance score — overall compliance % fetched from specsmith REST API
//!  2. Requirement coverage — number of covered vs total requirements
//!  3. Test coverage — number of tests vs linked requirements
//!  4. Gaps — list of uncovered requirements
//!  5. Refresh — button to re-fetch compliance data

use super::{
    settings_page::{
        build_sub_header, render_separator, MatchData, PageType, SettingsPageEvent,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use crate::themes::theme::Fill;
use kairos_governance::{GovernanceClient, GovernanceConfig};
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
// Compliance data state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
struct ComplianceData {
    overall_score: f64,
    total_requirements: usize,
    covered_requirements: usize,
    total_tests: usize,
    linked_tests: usize,
    gaps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum ComplianceStatus {
    Unknown,
    Loading,
    Loaded(ComplianceData),
    Error(String),
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum CompliancePageAction {
    Refresh,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct CompliancePageView {
    page: PageType<Self>,
    status: ComplianceStatus,
    refresh_button: MouseStateHandle,
}

impl CompliancePageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let mut view = CompliancePageView {
            page: PageType::new_monolith(CompliancePageWidget::default(), None, false),
            status: ComplianceStatus::Unknown,
            refresh_button: MouseStateHandle::default(),
        };
        view.fetch_compliance(ctx);
        view
    }

    fn fetch_compliance(&mut self, ctx: &mut ViewContext<Self>) {
        self.status = ComplianceStatus::Loading;
        ctx.notify();

        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                // Fetch compliance summary from specsmith REST API
                let resp = client.get_json("/api/compliance/summary").await;
                match resp {
                    Ok(json) => {
                        let data = ComplianceData {
                            overall_score: json
                                .get("overall_score")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0),
                            total_requirements: json
                                .get("total_requirements")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0)
                                as usize,
                            covered_requirements: json
                                .get("covered_requirements")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0)
                                as usize,
                            total_tests: json
                                .get("total_tests")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as usize,
                            linked_tests: json
                                .get("linked_tests")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as usize,
                            gaps: json
                                .get("gaps")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                                        .collect()
                                })
                                .unwrap_or_default(),
                        };
                        Ok(data)
                    }
                    Err(e) => Err(e),
                }
            },
            |me, result: Result<ComplianceData, anyhow::Error>, ctx| {
                me.status = match result {
                    Ok(data) => ComplianceStatus::Loaded(data),
                    Err(e) => ComplianceStatus::Error(format!("{e:#}")),
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
            CompliancePageAction::Refresh => {
                self.fetch_compliance(ctx);
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
                    Text::new_inline(label.to_string(), appearance.ui_font_family(), 13.)
                        .with_color(appearance.theme().disabled_ui_text_color().into())
                        .finish(),
                )
                .finish(),
            )
            .with_child(
                Text::new_inline(value.to_string(), appearance.monospace_font_family(), 14.)
                    .with_color(color.into())
                    .finish(),
            )
            .finish()
    }
}

impl SettingsWidget for CompliancePageWidget {
    type View = CompliancePageView;

    fn search_terms(&self) -> &str {
        "compliance requirements coverage tests gaps traceability audit governance"
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

        // ── Section 1: Overall compliance score ──────────────────────────
        let score_header = build_sub_header(appearance, "Compliance Score", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let score_card = match &view.status {
            ComplianceStatus::Unknown | ComplianceStatus::Loading => Self::card(
                Text::new(
                    "Loading compliance data\u{2026}".to_string(),
                    appearance.ui_font_family(),
                    13.,
                )
                .with_color(dim.into())
                .finish(),
                appearance,
            ),
            ComplianceStatus::Error(msg) => Self::card(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(
                        Text::new(
                            "Unable to fetch compliance data".to_string(),
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
                                "Start specsmith with: specsmith governance-serve".to_string(),
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
            ComplianceStatus::Loaded(data) => {
                let score_pct = format!("{:.0}%", data.overall_score * 100.0);
                let score_color = if data.overall_score >= 0.8 {
                    accent
                } else if data.overall_score >= 0.5 {
                    active
                } else {
                    Fill::Solid(theme.ui_error_color())
                };

                Self::card(
                    Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .with_child(
                            Flex::row()
                                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                .with_child(
                                    Text::new_inline(
                                        "\u{25CF}".to_string(),
                                        appearance.ui_font_family(),
                                        18.,
                                    )
                                    .with_color(score_color.into())
                                    .finish(),
                                )
                                .with_child(
                                    Container::new(
                                        Text::new_inline(
                                            format!("Overall: {score_pct}"),
                                            appearance.ui_font_family(),
                                            16.,
                                        )
                                        .with_color(active.into())
                                        .finish(),
                                    )
                                    .with_margin_left(10.)
                                    .finish(),
                                )
                                .finish(),
                        )
                        .with_child(
                            Container::new(Self::stat_row(
                                "Requirements covered",
                                &format!(
                                    "{} / {}",
                                    data.covered_requirements, data.total_requirements
                                ),
                                active,
                                appearance,
                            ))
                            .with_margin_top(12.)
                            .finish(),
                        )
                        .with_child(
                            Container::new(Self::stat_row(
                                "Tests linked",
                                &format!("{} / {}", data.linked_tests, data.total_tests),
                                active,
                                appearance,
                            ))
                            .with_margin_top(6.)
                            .finish(),
                        )
                        .finish(),
                    appearance,
                )
            }
        };

        // ── Section 2: Gaps ──────────────────────────────────────────────
        let gaps_header = build_sub_header(appearance, "Uncovered Requirements", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let gaps_card = match &view.status {
            ComplianceStatus::Loaded(data) if !data.gaps.is_empty() => {
                let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
                for (i, gap) in data.gaps.iter().take(20).enumerate() {
                    let label = Text::new(
                        format!("\u{2022}  {gap}"),
                        appearance.monospace_font_family(),
                        11.,
                    )
                    .with_color(active.into())
                    .soft_wrap(true)
                    .finish();
                    if i > 0 {
                        col.add_child(Container::new(label).with_margin_top(4.).finish());
                    } else {
                        col.add_child(label);
                    }
                }
                if data.gaps.len() > 20 {
                    col.add_child(
                        Container::new(
                            Text::new(
                                format!("… and {} more", data.gaps.len() - 20),
                                appearance.ui_font_family(),
                                11.,
                            )
                            .with_color(dim.into())
                            .finish(),
                        )
                        .with_margin_top(6.)
                        .finish(),
                    );
                }
                Self::card(col.finish(), appearance)
            }
            ComplianceStatus::Loaded(_) => Self::card(
                Text::new(
                    "\u{2714}  All requirements are covered".to_string(),
                    appearance.ui_font_family(),
                    13.,
                )
                .with_color(accent.into())
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

        // ── Refresh button ───────────────────────────────────────────────
        let refresh_button = appearance
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
                ctx.dispatch_typed_action(CompliancePageAction::Refresh);
            })
            .finish();

        // ── Assemble page ────────────────────────────────────────────────
        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(score_header)
                    .with_child(score_card)
                    .with_child(render_separator(appearance))
                    .with_child(gaps_header)
                    .with_child(gaps_card)
                    .with_child(render_separator(appearance))
                    .with_child(Container::new(refresh_button).with_margin_top(8.).finish())
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
        self.fetch_compliance(ctx);
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
