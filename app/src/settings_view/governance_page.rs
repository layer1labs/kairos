//! Settings page showing the local specsmith governance engine status.
//!
//! This page is read-only — all configuration happens through specsmith
//! itself or by pointing the BYOP endpoint to a different provider in
//! Settings → Agents → Providers.

use super::{
    settings_page::{MatchData, PageType, SettingsPageEvent, SettingsPageMeta,
                    SettingsPageViewHandle, SettingsWidget},
    SettingsSection,
};
use crate::appearance::Appearance;
use warpui::{
    elements::{
        ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, ParentElement,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

// ---------------------------------------------------------------------------
// Action (none needed — page is read-only for now)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GovernancePageAction {}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct GovernancePageView {
    page: PageType<Self>,
}

impl GovernancePageView {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        GovernancePageView {
            page: PageType::new_monolith(GovernancePageWidget::default(), None, false),
        }
    }
}

impl Entity for GovernancePageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for GovernancePageView {
    type Action = GovernancePageAction;

    fn handle_action(&mut self, action: &Self::Action, _ctx: &mut ViewContext<Self>) {
        match *action {}
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

impl SettingsWidget for GovernancePageWidget {
    type View = GovernancePageView;

    fn search_terms(&self) -> &str {
        "governance specsmith local ai engine byop endpoint port 7700"
    }

    fn render(
        &self,
        _view: &GovernancePageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let ui = appearance.ui_builder();

        let heading = ui
            .span("Local AI Governance")
            .build()
            .with_margin_bottom(8.)
            .finish();

        let status_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                ui.span("● specsmith governance-serve")
                    .build()
                    .finish(),
            )
            .finish();

        let endpoint = ui
            .span("BYOP endpoint  http://127.0.0.1:7700/v1/")
            .build()
            .with_margin_top(8.)
            .finish();

        let desc = ui
            .span(
                "Kairos spawns specsmith as a managed child process at startup. \
                 All AI governance — preflight checks, verification, confidence \
                 scoring, and audit — runs locally on your machine with no \
                 external network calls.",
            )
            .with_soft_wrap()
            .build()
            .with_margin_top(16.)
            .finish();

        let separator = Container::new(
            ConstrainedBox::new(warpui::elements::Empty::new().finish())
                .with_height(1.)
                .finish(),
        )
        .with_background_color(appearance.theme().outline().into_solid())
        .with_margin_top(20.)
        .with_margin_bottom(20.)
        .finish();

        let report_label = ui
            .span("Report governance bugs → github.com/BitConcepts/specsmith")
            .build()
            .finish();

        let report_terminal = ui
            .span("Report terminal bugs  → github.com/BitConcepts/kairos")
            .build()
            .with_margin_top(4.)
            .finish();

        let content = Flex::column()
            .with_child(heading)
            .with_child(status_row)
            .with_child(endpoint)
            .with_child(desc)
            .with_child(separator)
            .with_child(report_label)
            .with_child(report_terminal)
            .finish();

        Container::new(content)
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
