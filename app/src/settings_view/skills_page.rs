//! Skills management page — browse, build, activate AI agent skills.
//!
//! Sections:
//!  1. Skills list — fetched from specsmith `/api/skills`
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
struct SkillEntry {
    id: String,
    name: String,
    purpose: String,
    active: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum SkillsStatus {
    Unknown,
    Loading,
    Loaded(Vec<SkillEntry>),
    Error(String),
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SkillsPageAction {
    Refresh,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct SkillsPageView {
    page: PageType<Self>,
    status: SkillsStatus,
    refresh_button: MouseStateHandle,
}

impl SkillsPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let mut view = SkillsPageView {
            page: PageType::new_monolith(SkillsPageWidget::default(), None, false),
            status: SkillsStatus::Unknown,
            refresh_button: MouseStateHandle::default(),
        };
        view.fetch_skills(ctx);
        view
    }

    fn fetch_skills(&mut self, ctx: &mut ViewContext<Self>) {
        self.status = SkillsStatus::Loading;
        ctx.notify();

        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                let json = client.get_json("/api/skills").await?;
                let skills = json
                    .get("skills")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                Some(SkillEntry {
                                    id: v.get("id")?.as_str()?.to_owned(),
                                    name: v.get("name")?.as_str()?.to_owned(),
                                    purpose: v
                                        .get("purpose")
                                        .and_then(|p| p.as_str())
                                        .unwrap_or("")
                                        .to_owned(),
                                    active: v
                                        .get("active")
                                        .and_then(|a| a.as_bool())
                                        .unwrap_or(false),
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                Ok(skills)
            },
            |me, result: Result<Vec<SkillEntry>, anyhow::Error>, ctx| {
                me.status = match result {
                    Ok(skills) => SkillsStatus::Loaded(skills),
                    Err(e) => SkillsStatus::Error(format!("{e:#}")),
                };
                ctx.notify();
            },
        );
    }
}

impl Entity for SkillsPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for SkillsPageView {
    type Action = SkillsPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            SkillsPageAction::Refresh => self.fetch_skills(ctx),
        }
    }
}

impl View for SkillsPageView {
    fn ui_name() -> &'static str {
        "SkillsPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

#[derive(Default)]
struct SkillsPageWidget {}

impl SkillsPageWidget {
    fn card(content: Box<dyn Element>, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(content)
            .with_background(appearance.theme().surface_1())
            .with_uniform_padding(16.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_margin_bottom(12.)
            .finish()
    }
}

impl SettingsWidget for SkillsPageWidget {
    type View = SkillsPageView;

    fn search_terms(&self) -> &str {
        "skills agent build activate test ai automation"
    }

    fn render(
        &self,
        view: &SkillsPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let active_color = theme.active_ui_text_color();
        let accent: Fill = theme.accent().into_solid().into();

        let header = build_sub_header(appearance, "AI Agent Skills", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let content_card = match &view.status {
            SkillsStatus::Unknown | SkillsStatus::Loading => Self::card(
                Text::new(
                    "Loading skills\u{2026}".to_string(),
                    appearance.ui_font_family(),
                    13.,
                )
                .with_color(dim.into())
                .finish(),
                appearance,
            ),
            SkillsStatus::Error(msg) => Self::card(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(
                        Text::new(
                            "Unable to fetch skills".to_string(),
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
            SkillsStatus::Loaded(skills) if skills.is_empty() => Self::card(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(
                        Text::new(
                            "No skills configured".to_string(),
                            appearance.ui_font_family(),
                            13.,
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new(
                                "Create one: specsmith skills build \"<description>\"".to_string(),
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
            SkillsStatus::Loaded(skills) => {
                let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

                for (i, skill) in skills.iter().take(30).enumerate() {
                    let badge_color: Fill = if skill.active { accent } else { dim.into() };
                    let badge_text = if skill.active {
                        "\u{25CF} Active"
                    } else {
                        "\u{25CB} Inactive"
                    };

                    let row = Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .with_child(
                            Flex::row()
                                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                .with_child(
                                    Text::new_inline(
                                        badge_text.to_string(),
                                        appearance.ui_font_family(),
                                        11.,
                                    )
                                    .with_color(badge_color.into())
                                    .finish(),
                                )
                                .with_child(
                                    Container::new(
                                        Text::new_inline(
                                            skill.name.clone(),
                                            appearance.ui_font_family(),
                                            13.,
                                        )
                                        .with_color(active_color.into())
                                        .finish(),
                                    )
                                    .with_margin_left(10.)
                                    .finish(),
                                )
                                .finish(),
                        )
                        .with_child(
                            Container::new(
                                Text::new(
                                    skill.purpose.chars().take(120).collect::<String>(),
                                    appearance.monospace_font_family(),
                                    10.,
                                )
                                .with_color(dim.into())
                                .soft_wrap(true)
                                .finish(),
                            )
                            .with_margin_top(2.)
                            .finish(),
                        )
                        .finish();

                    if i > 0 {
                        col.add_child(Container::new(row).with_margin_top(10.).finish());
                    } else {
                        col.add_child(row);
                    }
                }

                if skills.len() > 30 {
                    col.add_child(
                        Container::new(
                            Text::new(
                                format!("\u{2026} and {} more", skills.len() - 30),
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
                ctx.dispatch_typed_action(SkillsPageAction::Refresh);
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

impl SettingsPageMeta for SkillsPageView {
    fn section() -> SettingsSection {
        SettingsSection::Skills
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn on_page_selected(&mut self, _: bool, ctx: &mut ViewContext<Self>) {
        self.fetch_skills(ctx);
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

impl From<ViewHandle<SkillsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<SkillsPageView>) -> Self {
        SettingsPageViewHandle::Skills(view_handle)
    }
}
