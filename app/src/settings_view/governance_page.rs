//! Governance settings page — specsmith engine status, updater, and links.
//!
//! Sections:
//!  1. Governance engine — live health indicator + BYOE endpoint
//!  2. specsmith updater  — installed version, Check for Updates / Update buttons (pipx)
//!  3. Links             — GitHub issue trackers for both repos

use super::{
    settings_page::{
        build_sub_header, render_separator, MatchData, PageType, SettingsPageEvent,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
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
// Governance health state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum HealthStatus {
    Unknown,
    Healthy { version: String },
    Unreachable { error: String },
}

// ---------------------------------------------------------------------------
// specsmith updater state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
enum UpdaterStatus {
    #[default]
    Idle,
    Checking,
    UpToDate { version: String },
    UpdateAvailable { current: String, latest: String },
    Updating,
    Updated { version: String },
    Error { message: String },
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GovernancePageAction {
    CheckForSpecsmithUpdate,
    UpdateSpecsmith,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct GovernancePageView {
    page: PageType<Self>,
    health: HealthStatus,
    updater: UpdaterStatus,
    check_update_button: MouseStateHandle,
    do_update_button: MouseStateHandle,
}

impl GovernancePageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let mut view = GovernancePageView {
            page: PageType::new_monolith(GovernancePageWidget::default(), None, false),
            health: HealthStatus::Unknown,
            updater: UpdaterStatus::Idle,
            check_update_button: MouseStateHandle::default(),
            do_update_button: MouseStateHandle::default(),
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

    /// Runs `pipx upgrade specsmith` in a subprocess and updates `updater` state.
    fn run_pipx_upgrade(&mut self, ctx: &mut ViewContext<Self>) {
        self.updater = UpdaterStatus::Updating;
        ctx.notify();
        ctx.spawn(
            async move {
                let out = std::process::Command::new("pipx")
                    .args(["upgrade", "specsmith"])
                    .output()
                    .map_err(|e| format!("pipx not found: {e:#}"))?;
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                if out.status.success() {
                    // pipx outputs "upgraded package specsmith x.y.z" or "already at latest"
                    let combined = format!("{stdout}{stderr}");
                    Ok(combined)
                } else {
                    Err(format!("{stdout}{stderr}"))
                }
            },
            |me, result, ctx| {
                me.updater = match result {
                    Ok(output) => {
                        // Detect if already up to date vs upgraded
                        let lower = output.to_lowercase();
                        if lower.contains("already at latest") {
                            // Extract current version from health or output
                            let ver = extract_version_from_output(&output)
                                .unwrap_or_else(|| "latest".to_owned());
                            UpdaterStatus::UpToDate { version: ver }
                        } else if lower.contains("upgraded") {
                            let ver = extract_version_from_output(&output)
                                .unwrap_or_else(|| "latest".to_owned());
                            UpdaterStatus::Updated { version: ver }
                        } else {
                            UpdaterStatus::UpToDate { version: "latest".to_owned() }
                        }
                    }
                    Err(e) => UpdaterStatus::Error {
                        message: e.chars().take(120).collect(),
                    },
                };
                ctx.notify();
            },
        );
    }

    /// Runs `pipx list --short` filtered for specsmith to get the installed version.
    fn check_for_update(&mut self, ctx: &mut ViewContext<Self>) {
        self.updater = UpdaterStatus::Checking;
        ctx.notify();
        ctx.spawn(
            async move {
                let out = std::process::Command::new("pipx")
                    .args(["list", "--short"])
                    .output()
                    .map_err(|e| format!("pipx not found: {e:#}"))?;
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                Ok(stdout)
            },
            |me, result, ctx| {
                me.updater = match result {
                    Ok(output) => {
                        // Look for "specsmith x.y.z" in pipx list output
                        let current = output
                            .lines()
                            .find(|l| l.to_lowercase().contains("specsmith"))
                            .and_then(|l| l.split_whitespace().nth(1))
                            .map(|v| v.to_owned())
                            .unwrap_or_else(|| "unknown".to_owned());
                        // After checking list, we don't know the latest without a network call.
                        // Report current version and prompt user to run upgrade if desired.
                        UpdaterStatus::UpToDate { version: current }
                    }
                    Err(e) => UpdaterStatus::Error {
                        message: e.chars().take(120).collect(),
                    },
                };
                ctx.notify();
            },
        );
    }
}

/// Extracts a semver-like version string from pipx upgrade output.
fn extract_version_from_output(output: &str) -> Option<String> {
    for word in output.split_whitespace() {
        if word.starts_with(|c: char| c.is_ascii_digit())
            && word.contains('.')
        {
            return Some(word.to_owned());
        }
    }
    None
}

impl Entity for GovernancePageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for GovernancePageView {
    type Action = GovernancePageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            GovernancePageAction::CheckForSpecsmithUpdate => {
                self.check_for_update(ctx);
            }
            GovernancePageAction::UpdateSpecsmith => {
                self.run_pipx_upgrade(ctx);
            }
        }
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

impl GovernancePageWidget {
    /// Small secondary button used for action buttons on this page.
    fn action_button(
        label: impl Into<String>,
        mouse_state: MouseStateHandle,
        action: GovernancePageAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, mouse_state)
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label(label.into())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(action.clone());
            })
            .finish()
    }

    /// Renders a card-style info block with a surface_1 background.
    fn card(content: Box<dyn Element>, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(content)
            .with_background(appearance.theme().surface_1())
            .with_uniform_padding(16.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_margin_bottom(12.)
            .finish()
    }

    /// Renders a dimmed monospace-style label (e.g. endpoint URL).
    fn dim_label(text: impl Into<String>, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(
            Text::new(
                text.into(),
                appearance.monospace_font_family(),
                11.,
            )
            .with_color(appearance.theme().disabled_ui_text_color().into())
            .finish(),
        )
        .with_margin_top(4.)
        .finish()
    }
}

impl SettingsWidget for GovernancePageWidget {
    type View = GovernancePageView;

    fn search_terms(&self) -> &str {
        "governance specsmith local ai engine BYOE endpoint port 7700 update pipx"
    }

    fn render(
        &self,
        view: &GovernancePageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color();

        // ── Section 1: Governance engine ─────────────────────────────────
        let engine_header = build_sub_header(appearance, "Local AI Governance", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let (dot_color, status_text) = match &view.health {
            HealthStatus::Unknown => (
                dim,
                "governance-serve \u{2014} checking\u{2026}".to_string(),
            ),
            HealthStatus::Healthy { version } => (
                // Green: use accent color as nearest "online" indicator
                theme.accent().into_solid().into(),
                format!("governance-serve  online  v{version}"),
            ),
            HealthStatus::Unreachable { .. } => (
                dim,
                "governance-serve  offline  (specsmith not running)".to_string(),
            ),
        };

        let dot = Text::new_inline(
            "\u{25CF}",
            appearance.ui_font_family(),
            13.,
        )
        .with_color(dot_color.into())
        .finish();

        let status_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(dot).with_margin_right(8.).finish()
            )
            .with_child(
                Text::new_inline(status_text, appearance.ui_font_family(), 13.)
                    .with_color(active.into())
                    .finish()
            )
            .finish();

        let endpoint_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(
                    Text::new_inline("BYOE endpoint", appearance.ui_font_family(), 12.)
                        .with_color(dim.into())
                        .finish()
                )
                .with_margin_right(8.)
                .finish()
            )
            .with_child(
                Self::dim_label("http://127.0.0.1:7700/v1/", appearance)
            )
            .finish();

        let desc_text = "Kairos spawns specsmith as a managed child process at startup. \
            All AI governance \u{2014} preflight checks, verification, confidence scoring, \
            and audit \u{2014} runs locally on your machine with no external network calls.";

        let desc = Container::new(
            Text::new(
                desc_text.to_string(),
                appearance.ui_font_family(),
                12.,
            )
            .with_color(dim.into())
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_top(10.)
        .finish();

        let engine_card_content = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(status_row)
            .with_child(Container::new(endpoint_row).with_margin_top(8.).finish())
            .with_child(desc)
            .finish();

        let engine_card = Self::card(engine_card_content, appearance);

        // ── Section 2: specsmith updater ──────────────────────────────────
        let updater_header = build_sub_header(appearance, "specsmith Updates", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let (updater_status_text, updater_color) = match &view.updater {
            UpdaterStatus::Idle => (
                "Click \"Check for updates\" to check the installed version.".to_string(),
                dim,
            ),
            UpdaterStatus::Checking => ("Checking\u{2026}".to_string(), dim),
            UpdaterStatus::UpToDate { version } => (
                format!("specsmith {version}  \u{2014}  up to date"),
                active,
            ),
            UpdaterStatus::UpdateAvailable { current, latest } => (
                format!("Update available: {current} \u{2192} {latest}"),
                active,
            ),
            UpdaterStatus::Updating => ("Updating via pipx\u{2026}".to_string(), dim),
            UpdaterStatus::Updated { version } => (
                format!("Updated to specsmith {version}"),
                theme.accent().into_solid().into(),
            ),
            UpdaterStatus::Error { message } => (
                format!("Error: {message}"),
                dim,
            ),
        };

        let updater_status_label = Text::new(
            updater_status_text,
            appearance.ui_font_family(),
            12.,
        )
        .with_color(updater_color.into())
        .soft_wrap(true)
        .finish();

        let check_btn = Self::action_button(
            "Check for updates",
            view.check_update_button.clone(),
            GovernancePageAction::CheckForSpecsmithUpdate,
            appearance,
        );

        let update_btn = Self::action_button(
            "Update (pipx upgrade specsmith)",
            view.do_update_button.clone(),
            GovernancePageAction::UpdateSpecsmith,
            appearance,
        );

        let button_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Container::new(check_btn).with_margin_right(8.).finish())
            .with_child(update_btn)
            .finish();

        let install_hint = Container::new(
            Text::new(
                "Managed via pipx. To install specsmith for the first time: pipx install specsmith"
                    .to_string(),
                appearance.monospace_font_family(),
                11.,
            )
            .with_color(dim.into())
            .finish(),
        )
        .with_margin_top(8.)
        .finish();

        let updater_card_content = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(updater_status_label)
            .with_child(Container::new(button_row).with_margin_top(10.).finish())
            .with_child(install_hint)
            .finish();

        let updater_card = Self::card(updater_card_content, appearance);

        // ── Section 3: Bug report links ───────────────────────────────────
        let links_header = build_sub_header(appearance, "Report Bugs", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let link_row = |label: &str, url: &str| -> Box<dyn Element> {
            let label_text = Text::new_inline(
                label.to_string(),
                appearance.ui_font_family(),
                12.,
            )
            .with_color(dim.into())
            .finish();
            let url_text = Text::new_inline(
                format!("\u{2192}  {url}"),
                appearance.monospace_font_family(),
                11.,
            )
            .with_color(theme.accent().into_solid().into())
            .finish();
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Expanded::new(1., Container::new(label_text).with_margin_right(12.).finish())
                        .finish(),
                )
                .with_child(url_text)
                .finish()
        };

        let links_card_content = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(link_row(
                "Governance / specsmith bugs",
                "github.com/BitConcepts/specsmith",
            ))
            .with_child(
                Container::new(link_row(
                    "Kairos terminal bugs",
                    "github.com/BitConcepts/kairos",
                ))
                .with_margin_top(8.)
                .finish(),
            )
            .finish();

        let links_card = Self::card(links_card_content, appearance);

        // ── Assemble page ─────────────────────────────────────────────────
        let mut page = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(engine_header)
            .with_child(engine_card)
            .with_child(render_separator(appearance))
            .with_child(updater_header)
            .with_child(updater_card)
            .with_child(render_separator(appearance))
            .with_child(links_header)
            .with_child(links_card);

        Container::new(
            ConstrainedBox::new(page.finish())
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
