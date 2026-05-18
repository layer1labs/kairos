//! Versioning and changelog settings page.
//!
//! Lets the user configure:
//!   - Version scheme: SemVer (default), CalVer, DateBuild, Custom
//!   - Changelog format: Keep a Changelog (default), Conventional Commits,
//!     GitHub Releases only, Manual, None

use super::{
    settings_page::{
        build_sub_header, render_separator, MatchData, PageType, SettingsPageEvent,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use warpui::{
    elements::{
        Container, CrossAxisAlignment, Element, Flex, MouseStateHandle, ParentElement, Text,
    },
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
    },
    AppContext, Entity, TypedActionView, View, ViewContext,
};

// ---------------------------------------------------------------------------
// Version scheme
// ---------------------------------------------------------------------------

/// Version numbering scheme for the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionScheme {
    /// MAJOR.MINOR.PATCH — semantic versioning.  **Default.**
    SemVer,
    /// YYYY.MM or YYYY.MM.DD — calendar versioning.
    CalVer,
    /// YYYYMMDD-NNN — date + build counter.
    DateBuild,
    /// Free-form version string defined by the project.
    Custom,
}

impl Default for VersionScheme {
    fn default() -> Self {
        Self::SemVer
    }
}

impl VersionScheme {
    pub fn label(self) -> &'static str {
        match self {
            Self::SemVer => "SemVer (1.2.3)",
            Self::CalVer => "CalVer (2024.01)",
            Self::DateBuild => "DateBuild (20240115-001)",
            Self::Custom => "Custom",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::SemVer => {
                "Semantic Versioning: MAJOR.MINOR.PATCH. \
                 Industry standard; compatible with pip, cargo, npm."
            }
            Self::CalVer => {
                "Calendar Versioning — date-based (e.g. Ubuntu 24.04, pip 24.0). \
                 Communicates release timeline rather than compatibility."
            }
            Self::DateBuild => {
                "Date + build counter. \
                 Used for rolling releases or daily builds."
            }
            Self::Custom => "Custom version pattern defined by the project.",
        }
    }

    fn as_file_str(self) -> &'static str {
        match self {
            Self::SemVer => "semver",
            Self::CalVer => "calver",
            Self::DateBuild => "datebuild",
            Self::Custom => "custom",
        }
    }

    fn from_str(s: &str) -> Self {
        match s.trim() {
            "calver" => Self::CalVer,
            "datebuild" => Self::DateBuild,
            "custom" => Self::Custom,
            _ => Self::SemVer,
        }
    }
}

// ---------------------------------------------------------------------------
// Changelog format
// ---------------------------------------------------------------------------

/// How changelogs are authored and maintained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangelogFormat {
    /// CHANGELOG.md following keepachangelog.com.  **Default.**
    KeepAChangelog,
    /// Auto-generated from conventional commit messages.
    ConventionalCommits,
    /// GitHub Releases only — no CHANGELOG.md in the repo.
    GitHubReleases,
    /// Manual CHANGELOG.md with no enforced format.
    Manual,
    /// No changelog.
    None,
}

impl Default for ChangelogFormat {
    fn default() -> Self {
        Self::KeepAChangelog
    }
}

impl ChangelogFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::KeepAChangelog => "Keep a Changelog",
            Self::ConventionalCommits => "Conventional Commits",
            Self::GitHubReleases => "GitHub Releases only",
            Self::Manual => "Manual",
            Self::None => "None",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::KeepAChangelog => {
                "CHANGELOG.md: Added, Changed, Deprecated, Removed, Fixed, Security. \
                 Human-authored per release."
            }
            Self::ConventionalCommits => {
                "Auto-generated from feat:, fix:, chore: commit messages. \
                 Compatible with conventional-changelog and release-please."
            }
            Self::GitHubReleases => {
                "GitHub Release notes only \u{2014} no CHANGELOG.md in the repo."
            }
            Self::Manual => "Manual CHANGELOG.md \u{2014} no enforced format.",
            Self::None => "No changelog. Not recommended for public projects.",
        }
    }

    fn as_file_str(self) -> &'static str {
        match self {
            Self::KeepAChangelog => "keep-a-changelog",
            Self::ConventionalCommits => "conventional-commits",
            Self::GitHubReleases => "github-releases",
            Self::Manual => "manual",
            Self::None => "none",
        }
    }

    fn from_str(s: &str) -> Self {
        match s.trim() {
            "conventional-commits" => Self::ConventionalCommits,
            "github-releases" => Self::GitHubReleases,
            "manual" => Self::Manual,
            "none" => Self::None,
            _ => Self::KeepAChangelog,
        }
    }
}

// ---------------------------------------------------------------------------
// Persisted config
// ---------------------------------------------------------------------------

pub struct VersioningConfig {
    pub scheme: VersionScheme,
    pub changelog: ChangelogFormat,
}

impl VersioningConfig {
    fn config_path() -> std::path::PathBuf {
        warp_core::paths::data_dir().join("kairos_version_config")
    }

    pub fn load() -> Self {
        let content = std::fs::read_to_string(Self::config_path()).unwrap_or_default();
        let mut scheme = VersionScheme::default();
        let mut changelog = ChangelogFormat::default();
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("scheme=") {
                scheme = VersionScheme::from_str(rest);
            } else if let Some(rest) = line.strip_prefix("changelog=") {
                changelog = ChangelogFormat::from_str(rest);
            }
        }
        Self { scheme, changelog }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(p) = path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let content = format!(
            "scheme={}\nchangelog={}\n",
            self.scheme.as_file_str(),
            self.changelog.as_file_str(),
        );
        let _ = std::fs::write(&path, content);
    }
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum VersioningPageAction {
    SetScheme(VersionScheme),
    SetChangelog(ChangelogFormat),
    OpenReleasePilotDocs,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct VersioningPageView {
    page: PageType<Self>,
    config: VersioningConfig,
    btn_semver: MouseStateHandle,
    btn_calver: MouseStateHandle,
    btn_datebuild: MouseStateHandle,
    btn_custom_scheme: MouseStateHandle,
    btn_keep_changelog: MouseStateHandle,
    btn_conv_commits: MouseStateHandle,
    btn_gh_releases: MouseStateHandle,
    btn_manual_cl: MouseStateHandle,
    btn_no_changelog: MouseStateHandle,
    btn_release_pilot: MouseStateHandle,
}

impl VersioningPageView {
    pub fn new(_ctx: &mut ViewContext<VersioningPageView>) -> Self {
        VersioningPageView {
            page: PageType::new_monolith(VersioningPageWidget::default(), None, false),
            config: VersioningConfig::load(),
            btn_semver: MouseStateHandle::default(),
            btn_calver: MouseStateHandle::default(),
            btn_datebuild: MouseStateHandle::default(),
            btn_custom_scheme: MouseStateHandle::default(),
            btn_keep_changelog: MouseStateHandle::default(),
            btn_conv_commits: MouseStateHandle::default(),
            btn_gh_releases: MouseStateHandle::default(),
            btn_manual_cl: MouseStateHandle::default(),
            btn_no_changelog: MouseStateHandle::default(),
            btn_release_pilot: MouseStateHandle::default(),
        }
    }
}

impl Entity for VersioningPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for VersioningPageView {
    type Action = VersioningPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            VersioningPageAction::SetScheme(scheme) => {
                self.config.scheme = *scheme;
                self.config.save();
                ctx.notify();
            }
            VersioningPageAction::SetChangelog(fmt) => {
                self.config.changelog = *fmt;
                self.config.save();
                ctx.notify();
            }
            VersioningPageAction::OpenReleasePilotDocs => {
                ctx.open_url(
                    "https://specsmith.readthedocs.io/en/latest/skills-index.html#governance-6",
                );
            }
        }
    }
}

impl View for VersioningPageView {
    fn ui_name() -> &'static str {
        "VersioningPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

#[derive(Default)]
struct VersioningPageWidget;

impl VersioningPageWidget {
    fn pill_btn<A: Clone + 'static>(
        label: &str,
        selected: bool,
        mouse_state: MouseStateHandle,
        action: A,
        appearance: &Appearance,
    ) -> Box<dyn Element>
    where
        A: warpui::Action,
    {
        let variant = if selected {
            ButtonVariant::Accent
        } else {
            ButtonVariant::Secondary
        };
        appearance
            .ui_builder()
            .button(variant, mouse_state)
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label(label.to_string())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(action.clone());
            })
            .finish()
    }
}

impl SettingsWidget for VersioningPageWidget {
    type View = VersioningPageView;

    fn search_terms(&self) -> &str {
        "versioning version scheme semver calver changelog keep-a-changelog \
         conventional-commits release changelog format"
    }

    fn render(
        &self,
        view: &VersioningPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let cur_scheme = view.config.scheme;
        let cur_changelog = view.config.changelog;

        // ── Version scheme ─────────────────────────────────────────────
        let scheme_header = build_sub_header(appearance, "Version Scheme", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let scheme_desc = Text::new(
            cur_scheme.description().to_string(),
            appearance.ui_font_family(),
            12.,
        )
        .with_color(dim.into())
        .soft_wrap(true)
        .finish();

        let scheme_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(Self::pill_btn(
                    VersionScheme::SemVer.label(),
                    cur_scheme == VersionScheme::SemVer,
                    view.btn_semver.clone(),
                    VersioningPageAction::SetScheme(VersionScheme::SemVer),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::pill_btn(
                    VersionScheme::CalVer.label(),
                    cur_scheme == VersionScheme::CalVer,
                    view.btn_calver.clone(),
                    VersioningPageAction::SetScheme(VersionScheme::CalVer),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::pill_btn(
                    VersionScheme::DateBuild.label(),
                    cur_scheme == VersionScheme::DateBuild,
                    view.btn_datebuild.clone(),
                    VersioningPageAction::SetScheme(VersionScheme::DateBuild),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(Self::pill_btn(
                VersionScheme::Custom.label(),
                cur_scheme == VersionScheme::Custom,
                view.btn_custom_scheme.clone(),
                VersioningPageAction::SetScheme(VersionScheme::Custom),
                appearance,
            ))
            .finish();

        // ── Changelog format ───────────────────────────────────────────
        let changelog_header = build_sub_header(appearance, "Changelog Format", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let changelog_desc = Text::new(
            cur_changelog.description().to_string(),
            appearance.ui_font_family(),
            12.,
        )
        .with_color(dim.into())
        .soft_wrap(true)
        .finish();

        let changelog_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(Self::pill_btn(
                    ChangelogFormat::KeepAChangelog.label(),
                    cur_changelog == ChangelogFormat::KeepAChangelog,
                    view.btn_keep_changelog.clone(),
                    VersioningPageAction::SetChangelog(ChangelogFormat::KeepAChangelog),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::pill_btn(
                    ChangelogFormat::ConventionalCommits.label(),
                    cur_changelog == ChangelogFormat::ConventionalCommits,
                    view.btn_conv_commits.clone(),
                    VersioningPageAction::SetChangelog(ChangelogFormat::ConventionalCommits),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::pill_btn(
                    ChangelogFormat::GitHubReleases.label(),
                    cur_changelog == ChangelogFormat::GitHubReleases,
                    view.btn_gh_releases.clone(),
                    VersioningPageAction::SetChangelog(ChangelogFormat::GitHubReleases),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::pill_btn(
                    ChangelogFormat::Manual.label(),
                    cur_changelog == ChangelogFormat::Manual,
                    view.btn_manual_cl.clone(),
                    VersioningPageAction::SetChangelog(ChangelogFormat::Manual),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(Self::pill_btn(
                ChangelogFormat::None.label(),
                cur_changelog == ChangelogFormat::None,
                view.btn_no_changelog.clone(),
                VersioningPageAction::SetChangelog(ChangelogFormat::None),
                appearance,
            ))
            .finish();

        // ── Release pilot hint ─────────────────────────────────────────
        let pilot_hint = Text::new(
            "Use the `release-pilot` specsmith skill to automate version bumps \
             and CHANGELOG.md updates in your release workflow."
                .to_string(),
            appearance.ui_font_family(),
            12.,
        )
        .with_color(dim.into())
        .soft_wrap(true)
        .finish();

        let pilot_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.btn_release_pilot.clone())
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label("Release Pilot Skill Docs \u{2197}".to_string())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(VersioningPageAction::OpenReleasePilotDocs);
            })
            .finish();

        Container::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(scheme_header)
                .with_child(Container::new(scheme_desc).with_margin_bottom(12.).finish())
                .with_child(Container::new(scheme_row).with_margin_bottom(12.).finish())
                .with_child(render_separator(appearance))
                .with_child(changelog_header)
                .with_child(
                    Container::new(changelog_desc)
                        .with_margin_bottom(12.)
                        .finish(),
                )
                .with_child(
                    Container::new(changelog_row)
                        .with_margin_bottom(12.)
                        .finish(),
                )
                .with_child(render_separator(appearance))
                .with_child(Container::new(pilot_hint).with_margin_bottom(8.).finish())
                .with_child(pilot_btn)
                .finish(),
        )
        .with_uniform_padding(28.)
        .finish()
    }
}

// ---------------------------------------------------------------------------
// SettingsPageMeta
// ---------------------------------------------------------------------------

impl SettingsPageMeta for VersioningPageView {
    fn section() -> SettingsSection {
        SettingsSection::Versioning
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id);
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<warpui::ViewHandle<VersioningPageView>> for SettingsPageViewHandle {
    fn from(view_handle: warpui::ViewHandle<VersioningPageView>) -> Self {
        SettingsPageViewHandle::Versioning(view_handle)
    }
}
