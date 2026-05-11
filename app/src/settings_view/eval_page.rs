//! Eval dashboard page — browse and run eval suites for AI model benchmarking.
//!
//! Sections:
//!  1. Eval suites — fetched from specsmith `/api/eval/suites`
//!  2. Refresh button

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
        ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element, Flex,
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
struct EvalSuiteEntry {
    id: String,
    name: String,
    description: String,
    case_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum EvalStatus {
    Unknown,
    Loading,
    Loaded(Vec<EvalSuiteEntry>),
    Error(String),
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum EvalPageAction {
    Refresh,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct EvalPageView {
    page: PageType<Self>,
    status: EvalStatus,
    refresh_button: MouseStateHandle,
}

impl EvalPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let mut view = EvalPageView {
            page: PageType::new_monolith(EvalPageWidget::default(), None, false),
            status: EvalStatus::Unknown,
            refresh_button: MouseStateHandle::default(),
        };
        view.fetch_suites(ctx);
        view
    }

    fn fetch_suites(&mut self, ctx: &mut ViewContext<Self>) {
        self.status = EvalStatus::Loading;
        ctx.notify();

        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                let json = client.get_json("/api/eval/suites").await?;
                let suites = json
                    .get("suites")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                let cases = v
                                    .get("cases")
                                    .and_then(|c| c.as_array())
                                    .map(|a| a.len())
                                    .unwrap_or(0);
                                Some(EvalSuiteEntry {
                                    id: v.get("id")?.as_str()?.to_owned(),
                                    name: v.get("name")?.as_str()?.to_owned(),
                                    description: v
                                        .get("description")
                                        .and_then(|d| d.as_str())
                                        .unwrap_or("")
                                        .to_owned(),
                                    case_count: cases,
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                Ok(suites)
            },
            |me, result: Result<Vec<EvalSuiteEntry>, anyhow::Error>, ctx| {
                me.status = match result {
                    Ok(suites) => EvalStatus::Loaded(suites),
                    Err(e) => EvalStatus::Error(format!("{e:#}")),
                };
                ctx.notify();
            },
        );
    }
}

impl Entity for EvalPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for EvalPageView {
    type Action = EvalPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            EvalPageAction::Refresh => self.fetch_suites(ctx),
        }
    }
}

impl View for EvalPageView {
    fn ui_name() -> &'static str {
        "EvalPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

#[derive(Default)]
struct EvalPageWidget {}

impl EvalPageWidget {
    fn card(content: Box<dyn Element>, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(content)
            .with_background(appearance.theme().surface_1())
            .with_uniform_padding(16.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_margin_bottom(12.)
            .finish()
    }
}

impl SettingsWidget for EvalPageWidget {
    type View = EvalPageView;

    fn search_terms(&self) -> &str {
        "eval evaluation benchmark suite model test score latency"
    }

    fn render(
        &self,
        view: &EvalPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let active_color = theme.active_ui_text_color();
        let accent: Fill = theme.accent().into_solid().into();

        let header = build_sub_header(appearance, "Eval Dashboard", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let content_card = match &view.status {
            EvalStatus::Unknown | EvalStatus::Loading => Self::card(
                Text::new(
                    "Loading eval suites\u{2026}".to_string(),
                    appearance.ui_font_family(),
                    13.,
                )
                .with_color(dim.into())
                .finish(),
                appearance,
            ),
            EvalStatus::Error(msg) => Self::card(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(
                        Text::new(
                            "Unable to fetch eval suites".to_string(),
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
            EvalStatus::Loaded(suites) if suites.is_empty() => Self::card(
                Text::new(
                    "No eval suites available".to_string(),
                    appearance.ui_font_family(),
                    13.,
                )
                .with_color(dim.into())
                .finish(),
                appearance,
            ),
            EvalStatus::Loaded(suites) => {
                let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

                for (i, suite) in suites.iter().enumerate() {
                    let row = Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .with_child(
                            Flex::row()
                                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                .with_child(
                                    Text::new_inline(
                                        "\u{25B6}".to_string(),
                                        appearance.ui_font_family(),
                                        12.,
                                    )
                                    .with_color(accent.into())
                                    .finish(),
                                )
                                .with_child(
                                    Container::new(
                                        Text::new_inline(
                                            suite.name.clone(),
                                            appearance.ui_font_family(),
                                            14.,
                                        )
                                        .with_color(active_color.into())
                                        .finish(),
                                    )
                                    .with_margin_left(10.)
                                    .finish(),
                                )
                                .with_child(
                                    Container::new(
                                        Text::new_inline(
                                            format!("{} cases", suite.case_count),
                                            appearance.monospace_font_family(),
                                            11.,
                                        )
                                        .with_color(dim.into())
                                        .finish(),
                                    )
                                    .with_margin_left(12.)
                                    .finish(),
                                )
                                .finish(),
                        )
                        .with_child(
                            Container::new(
                                Text::new(
                                    suite.description.chars().take(150).collect::<String>(),
                                    appearance.ui_font_family(),
                                    11.,
                                )
                                .with_color(dim.into())
                                .soft_wrap(true)
                                .finish(),
                            )
                            .with_margin_top(4.)
                            .finish(),
                        )
                        .with_child(
                            Container::new(
                                Text::new(
                                    format!("Run: specsmith eval run {}", suite.id),
                                    appearance.monospace_font_family(),
                                    10.,
                                )
                                .with_color(dim.into())
                                .finish(),
                            )
                            .with_margin_top(4.)
                            .finish(),
                        )
                        .finish();

                    if i > 0 {
                        col.add_child(Container::new(row).with_margin_top(12.).finish());
                    } else {
                        col.add_child(row);
                    }
                }

                Self::card(col.finish(), appearance)
            }
        };

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
                ctx.dispatch_typed_action(EvalPageAction::Refresh);
            })
            .finish();

        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(header)
                    .with_child(content_card)
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

impl SettingsPageMeta for EvalPageView {
    fn section() -> SettingsSection {
        SettingsSection::Eval
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn on_page_selected(&mut self, _: bool, ctx: &mut ViewContext<Self>) {
        self.fetch_suites(ctx);
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

impl From<ViewHandle<EvalPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<EvalPageView>) -> Self {
        SettingsPageViewHandle::Eval(view_handle)
    }
}
