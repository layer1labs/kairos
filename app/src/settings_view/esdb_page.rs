//! ESDB (Epistemic State Database) dashboard page — ChronoMemory status.
//!
//! Sections:
//!  1. ESDB status — backend type, record count, chain integrity
//!  2. Record counts — facts, hypotheses, requirements, tests, etc.

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
// State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
struct EsdbData {
    available: bool,
    backend: String,
    record_count: usize,
    requirements: usize,
    testcases: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum EsdbStatus {
    Unknown,
    Loading,
    Loaded(EsdbData),
    Error(String),
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum EsdbPageAction {
    Refresh,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct EsdbPageView {
    page: PageType<Self>,
    status: EsdbStatus,
    refresh_button: MouseStateHandle,
}

impl EsdbPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let mut view = EsdbPageView {
            page: PageType::new_monolith(EsdbPageWidget::default(), None, false),
            status: EsdbStatus::Unknown,
            refresh_button: MouseStateHandle::default(),
        };
        view.fetch_esdb(ctx);
        view
    }

    fn fetch_esdb(&mut self, ctx: &mut ViewContext<Self>) {
        self.status = EsdbStatus::Loading;
        ctx.notify();

        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                let status_json = client.get_json("/api/esdb/status").await?;
                let counts_json = client.get_json("/api/esdb/counts").await;
                let data = EsdbData {
                    available: status_json
                        .get("available")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    backend: status_json
                        .get("backend")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_owned(),
                    record_count: status_json
                        .get("record_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    requirements: counts_json
                        .as_ref()
                        .ok()
                        .and_then(|j| j.get("requirements"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    testcases: counts_json
                        .as_ref()
                        .ok()
                        .and_then(|j| j.get("testcases"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                };
                Ok(data)
            },
            |me, result: Result<EsdbData, anyhow::Error>, ctx| {
                me.status = match result {
                    Ok(data) => EsdbStatus::Loaded(data),
                    Err(e) => EsdbStatus::Error(format!("{e:#}")),
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
            EsdbPageAction::Refresh => self.fetch_esdb(ctx),
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
                    Text::new_inline(label.to_string(), appearance.ui_font_family(), 13.)
                        .with_color(appearance.theme().disabled_ui_text_color().into())
                        .finish(),
                )
                .finish(),
            )
            .with_child(
                Text::new_inline(value.to_string(), appearance.monospace_font_family(), 14.)
                    .with_color(appearance.theme().active_ui_text_color().into())
                    .finish(),
            )
            .finish()
    }
}

impl SettingsWidget for EsdbPageWidget {
    type View = EsdbPageView;

    fn search_terms(&self) -> &str {
        "esdb chronomemory epistemic state database facts hypotheses requirements tests rollback replay dependency"
    }

    fn render(
        &self,
        view: &EsdbPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let accent: Fill = theme.accent().into_solid().into();

        let header = build_sub_header(appearance, "ChronoMemory ESDB", None)
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
                let status_color = if data.available { accent } else { dim };
                let status_text = if data.available {
                    format!("\u{25CF} Online \u{2014} {}", data.backend)
                } else {
                    "\u{25CF} Offline".to_string()
                };

                Self::card(
                    Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .with_child(
                            Text::new_inline(status_text, appearance.ui_font_family(), 14.)
                                .with_color(status_color.into())
                                .finish(),
                        )
                        .with_child(
                            Container::new(Self::stat_row(
                                "Total records",
                                &data.record_count.to_string(),
                                appearance,
                            ))
                            .with_margin_top(10.)
                            .finish(),
                        )
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
                        .finish(),
                    appearance,
                )
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
                ctx.dispatch_typed_action(EsdbPageAction::Refresh);
            })
            .finish();

        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(header)
                    .with_child(status_card)
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

impl SettingsPageMeta for EsdbPageView {
    fn section() -> SettingsSection {
        SettingsSection::Esdb
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn on_page_selected(&mut self, _: bool, ctx: &mut ViewContext<Self>) {
        self.fetch_esdb(ctx);
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
