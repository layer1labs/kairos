//! Settings page showing the local specsmith governance engine status.
//!
//! Polls `GET /health` on `specsmith serve` every 5 seconds when the page is
//! visible and shows a live green/red status indicator.

use super::{
    settings_page::{
        MatchData, PageType, SettingsPageEvent, SettingsPageMeta, SettingsPageViewHandle,
        SettingsWidget,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use kairos_governance::{GovernanceClient, GovernanceConfig};
use warpui::{
    elements::{
        ConstrainedBox, Container, CrossAxisAlignment, Element, Empty, Flex, ParentElement,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

// ---------------------------------------------------------------------------
// Health state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum HealthStatus {
    Unknown,
    Healthy { version: String },
    Unreachable { error: String },
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GovernancePageAction {}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct GovernancePageView {
    page: PageType<Self>,
    health: HealthStatus,
}

impl GovernancePageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let mut view = GovernancePageView {
            page: PageType::new_monolith(GovernancePageWidget::default(), None, false),
            health: HealthStatus::Unknown,
        };
        view.check_health(ctx);
        view
    }

    fn check_health(&mut self, ctx: &mut ViewContext<Self>) {
        let config = GovernanceConfig::default_local();
        ctx.spawn(
            async move {
                let client = GovernanceClient::new(config)?;
                client.health().await
            },
            |me, result, ctx| {
                me.health = match result {
                    Ok(resp) => HealthStatus::Healthy {
                        version: resp.version,
                    },
                    Err(e) => HealthStatus::Unreachable {
                        error: format!("{e:#}"),
                    },
                };
                ctx.notify();
            },
        );
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
        "governance specsmith local ai engine BYOE endpoint port 7700"
    }

    fn render(
        &self,
        view: &GovernancePageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let ui = appearance.ui_builder();

        let heading = ui
            .span("Local AI Governance")
            .build()
            .with_margin_bottom(8.)
            .finish();

        // Dynamic status indicator based on health polling result.
        let (indicator, status_text) = match &view.health {
            HealthStatus::Unknown => (
                "\u{25CC}",
                "specsmith governance-serve \u{2014} checking\u{2026}".to_string(),
            ),
            HealthStatus::Healthy { version } => (
                "\u{25CF}",
                format!("specsmith governance-serve — online (v{version})"),
            ),
            HealthStatus::Unreachable { error } => (
                "\u{25CB}",
                format!("specsmith governance-serve — offline ({error})"),
            ),
        };

        let status_label = format!("{indicator} {status_text}");
        let status_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(ui.span(status_label).build().finish())
            .finish();

        let endpoint = ui
            .span("BYOE endpoint  http://127.0.0.1:7700/v1/")
            .build()
            .with_margin_top(8.)
            .finish();

        let desc = ui
            .span(
                "Kairos spawns specsmith as a managed child process at startup. \
                 All AI governance \u{2014} preflight checks, verification, confidence \
                 scoring, and audit \u{2014} runs locally on your machine with no \
                 external network calls.",
            )
            .with_soft_wrap()
            .build()
            .with_margin_top(16.)
            .finish();

        let separator = Container::new(
            ConstrainedBox::new(Empty::new().finish())
                .with_height(1.)
                .finish(),
        )
        .with_background_color(appearance.theme().outline().into_solid())
        .with_margin_top(20.)
        .with_margin_bottom(20.)
        .finish();

        let report_label = ui
            .span("Report governance bugs \u{2192} github.com/BitConcepts/specsmith")
            .build()
            .finish();

        let report_terminal = ui
            .span("Report terminal bugs  \u{2192} github.com/BitConcepts/kairos")
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

        Container::new(content).with_uniform_padding(28.).finish()
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

    fn on_page_selected(&mut self, _: bool, ctx: &mut ViewContext<Self>) {
        // Refresh immediately when user navigates to this page.
        self.check_health(ctx);
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
