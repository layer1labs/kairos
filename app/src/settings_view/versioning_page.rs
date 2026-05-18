//! Versioning and changelog settings page.
//!
//! Lets the user configure:
//!   - **Version scheme** — SemVer (default), CalVer, DateBuild, or Custom.
//!   - **Changelog format** — Keep a Changelog (default), Conventional Commits,
//!     GitHub Releases only, Manual, or None.
//!
//! Both choices are persisted to `{data_dir}/kairos_version_config` and
//! surfaced to specsmith's release-pilot skill when generating changelogs.

use super::{
    settings_page::{
        build_sub_header, render_separator, MatchData, PageType, SettingsPageEvent,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use warpui::{
    elements::{Container, CrossAxisAlignment, Element, Flex, MouseStateHandle, ParentElement},
    ui_components::{
        button::ButtonVariant,
        components::{UiComponent, UiComponentStyles},
    },
    AppContext, Entity, TypedActionView, View, ViewContext,
};
// NOTE: SettingsPageViewHandle imported indirectly via settings_page macro — not used directly.

// ---------------------------------------------------------------------------
// Version scheme
// ---------------------------------------------------------------------------

/// Version numbering scheme for the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionScheme {
    /// `MAJOR.MINOR.PATCH` — semantic versioning.  **Default.**
    SemVer,
    /// `YYYY.MM.DD` or `YYYY.0M` calendar versioning.
    CalVer,
    /// `YYYYMMDD-NNN` — date + daily build counter.
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
                "Semantic Versioning — MAJOR.MINOR.PATCH. \
                 Industry standard. Compatible with pip, cargo, npm. \
                 MAJOR: breaking change; MINOR: new feature; PATCH: fix."
            }
            Self::CalVer => {
                "Calendar Versioning — date-based. \
                 Examples: Ubuntu 24.04, pip 24.0, Black 24.1. \
                 Communicates release timeline rather than compatibility."
            }
            Self::DateBuild => {
                "Date + build counter. \
                 Used for rolling releases or daily builds where semantic \
                 compatibility is not the primary signal."
            }
            Self::Custom => {
                "Custom version pattern defined by the project. \
                 Specify the pattern in your project configuration."
            }
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
    /// `CHANGELOG.md` following [keepachangelog.com](https://keepachangelog.com).  **Default.**
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
                "CHANGELOG.md with sections: Added, Changed, Deprecated, \
                 Removed, Fixed, Security. Human-authored per release."
            }
            Self::ConventionalCommits => {
                "Auto-generated from `feat:`, `fix:`, `chore:` commit messages. \
                 Compatible with conventional-changelog and release-please."
            }
            Self::GitHubReleases => {
                "Use GitHub Release notes only — no CHANGELOG.md in the repo. \
                 Good for projects where all releases go through GitHub."
            }
            Self::Manual => {
                "Manual CHANGELOG.md — no enforced format. \
                 Author entries however you like."
            }
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
                    "https://specsmith.readthedocs.io/en/latest/skills-index.html\
                     #governance-6",
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
        let ui = appearance.ui_builder();
        let cur_scheme = view.config.scheme;
        let cur_changelog = view.config.changelog;

        // ── Version scheme ────────────────────────────────────────────────────

        let scheme_header = build_sub_header(appearance, "Version Scheme", None).finish();

        let scheme_desc = ui
            .span(cur_scheme.description().to_string())
            .build()
            .with_margin_bottom(8.)
            .finish();

        let make_scheme_pill =
            |label: &str, scheme: VersionScheme, selected: bool, handle: MouseStateHandle| {
                let variant = if selected {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                };
                ui.button(label, variant)
                    .build()
                    .mouse_state(handle)
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(VersioningPageAction::SetScheme(scheme));
                    })
                    .with_margin_right(6.)
                    .finish()
            };

        let scheme_row = Flex::row()
            .with_children([
                make_scheme_pill(
                    VersionScheme::SemVer.label(),
                    VersionScheme::SemVer,
                    cur_scheme == VersionScheme::SemVer,
                    view.btn_semver.clone(),
                ),
                make_scheme_pill(
                    VersionScheme::CalVer.label(),
                    VersionScheme::CalVer,
                    cur_scheme == VersionScheme::CalVer,
                    view.btn_calver.clone(),
                ),
                make_scheme_pill(
                    VersionScheme::DateBuild.label(),
                    VersionScheme::DateBuild,
                    cur_scheme == VersionScheme::DateBuild,
                    view.btn_datebuild.clone(),
                ),
                make_scheme_pill(
                    VersionScheme::Custom.label(),
                    VersionScheme::Custom,
                    cur_scheme == VersionScheme::Custom,
                    view.btn_custom_scheme.clone(),
                ),
            ])
            .finish();

        // ── Changelog format ─────────────────────────────────────────────────

        let sep = render_separator(appearance);
        let changelog_header = build_sub_header(appearance, "Changelog Format", None).finish();

        let changelog_desc = ui
            .span(cur_changelog.description().to_string())
            .build()
            .with_margin_bottom(8.)
            .finish();

        let make_cl_pill =
            |label: &str, fmt: ChangelogFormat, selected: bool, handle: MouseStateHandle| {
                let variant = if selected {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                };
                ui.button(label, variant)
                    .build()
                    .mouse_state(handle)
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(VersioningPageAction::SetChangelog(fmt));
                    })
                    .with_margin_right(6.)
                    .finish()
            };

        let changelog_row = Flex::row()
            .with_children([
                make_cl_pill(
                    ChangelogFormat::KeepAChangelog.label(),
                    ChangelogFormat::KeepAChangelog,
                    cur_changelog == ChangelogFormat::KeepAChangelog,
                    view.btn_keep_changelog.clone(),
                ),
                make_cl_pill(
                    ChangelogFormat::ConventionalCommits.label(),
                    ChangelogFormat::ConventionalCommits,
                    cur_changelog == ChangelogFormat::ConventionalCommits,
                    view.btn_conv_commits.clone(),
                ),
                make_cl_pill(
                    ChangelogFormat::GitHubReleases.label(),
                    ChangelogFormat::GitHubReleases,
                    cur_changelog == ChangelogFormat::GitHubReleases,
                    view.btn_gh_releases.clone(),
                ),
                make_cl_pill(
                    ChangelogFormat::Manual.label(),
                    ChangelogFormat::Manual,
                    cur_changelog == ChangelogFormat::Manual,
                    view.btn_manual_cl.clone(),
                ),
                make_cl_pill(
                    ChangelogFormat::None.label(),
                    ChangelogFormat::None,
                    cur_changelog == ChangelogFormat::None,
                    view.btn_no_changelog.clone(),
                ),
            ])
            .finish();

        // ── Release Pilot hint ────────────────────────────────────────────────

        let sep2 = render_separator(appearance);
        let pilot_hint = ui
            .span(
                "Use the `release-pilot` skill to automate version bumps and \
                 CHANGELOG.md updates in your release workflow."
                    .to_string(),
            )
            .build()
            .with_margin_bottom(8.)
            .finish();

        let pilot_button = ui
            .button("Open Release Pilot Skill Docs", ButtonVariant::Secondary)
            .build()
            .mouse_state(view.btn_release_pilot.clone())
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(VersioningPageAction::OpenReleasePilotDocs);
            })
            .finish();

        // ── Assemble ──────────────────────────────────────────────────────────

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(
                Container::new(scheme_header)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(scheme_desc)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(scheme_row)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(sep)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(changelog_header)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(changelog_desc)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(changelog_row)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(sep2)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(pilot_hint)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(pilot_button)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
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

    fn match_data() -> MatchData {
        MatchData {
            additional_match_data: Some("versioning version semver calver changelog release"),
        }
    }
}
