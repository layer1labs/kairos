//! Documentation settings page — choose a documentation system for the current project.
//!
//! Governance: H23 Documentation Lifecycle Gate, H24 Skills Documentation Required.
//!
//! The user selects a documentation system for their project.  The selection is
//! persisted to `{data_dir}/kairos_doc_system` and surfaced to specsmith so the
//! correct documentation skill is suggested during project governance checks.
//!
//! Default: UserManual (MANUAL.md) — always produces *something*; never None.

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

// ---------------------------------------------------------------------------
// Documentation system model
// ---------------------------------------------------------------------------

/// Which documentation system is configured for this project.
///
/// The default is [`DocSystem::UserManual`] — a single MANUAL.md file.
/// `None` is intentionally last and not the default; every project should
/// have at least a minimal user manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocSystem {
    /// MANUAL.md — single-file bare-minimum docs.  **Suggested default.**
    UserManual,
    /// MkDocs with Material theme + ReadTheDocs.  Default for Python projects.
    MkDocs,
    /// Sphinx with autodoc + MyST.  For Python API documentation.
    Sphinx,
    /// mdBook — Rust application/project documentation.
    MdBook,
    /// rustdoc / cargo doc — Rust library API documentation.
    Rustdoc,
    /// Doxygen — C, C++, VHDL/Verilog multi-language documentation.
    Doxygen,
    /// JSDoc — JavaScript API documentation.
    JsDoc,
    /// TypeDoc — TypeScript API documentation.
    TypeDoc,
    /// Javadoc / Dokka — Java and Kotlin documentation.
    Javadoc,
    /// OpenAPI / Swagger — REST API specification and docs.
    OpenApi,
    /// No documentation system configured.
    None,
}

impl Default for DocSystem {
    /// Default is UserManual — never None.
    ///
    /// NOTE — first stable release: consider making the default project-type-aware.
    fn default() -> Self {
        Self::UserManual
    }
}

impl DocSystem {
    pub fn label(self) -> &'static str {
        match self {
            Self::UserManual => "User Manual (MANUAL.md)",
            Self::MkDocs => "MkDocs + Material / RTD",
            Self::Sphinx => "Sphinx / MyST",
            Self::MdBook => "mdBook (Rust)",
            Self::Rustdoc => "rustdoc / cargo doc",
            Self::Doxygen => "Doxygen (C/C++/VHDL)",
            Self::JsDoc => "JSDoc (JavaScript)",
            Self::TypeDoc => "TypeDoc (TypeScript)",
            Self::Javadoc => "Javadoc / Dokka (Java/Kotlin)",
            Self::OpenApi => "OpenAPI / Swagger",
            Self::None => "None",
        }
    }

    pub fn skill_slug(self) -> Option<&'static str> {
        match self {
            Self::UserManual => Some("user-manual-md"),
            Self::MkDocs => Some("mkdocs"),
            Self::Sphinx => Some("sphinx"),
            Self::MdBook => Some("mdbook"),
            Self::Rustdoc => Some("rustdoc"),
            Self::Doxygen => Some("doxygen"),
            Self::JsDoc => Some("jsdoc"),
            Self::TypeDoc => Some("typedoc"),
            Self::Javadoc => Some("javadoc"),
            Self::OpenApi => Some("openapi"),
            Self::None => None,
        }
    }

    fn as_file_str(self) -> &'static str {
        match self {
            Self::UserManual => "user-manual",
            Self::MkDocs => "mkdocs",
            Self::Sphinx => "sphinx",
            Self::MdBook => "mdbook",
            Self::Rustdoc => "rustdoc",
            Self::Doxygen => "doxygen",
            Self::JsDoc => "jsdoc",
            Self::TypeDoc => "typedoc",
            Self::Javadoc => "javadoc",
            Self::OpenApi => "openapi",
            Self::None => "none",
        }
    }

    fn from_str(s: &str) -> Self {
        match s.trim() {
            "mkdocs" => Self::MkDocs,
            "sphinx" => Self::Sphinx,
            "mdbook" => Self::MdBook,
            "rustdoc" => Self::Rustdoc,
            "doxygen" => Self::Doxygen,
            "jsdoc" => Self::JsDoc,
            "typedoc" => Self::TypeDoc,
            "javadoc" => Self::Javadoc,
            "openapi" => Self::OpenApi,
            "none" => Self::None,
            _ => Self::UserManual, // default for unknown/missing
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::UserManual => {
                "A single MANUAL.md at the project root. Bare minimum \
                 documentation — always a safe default."
            }
            Self::MkDocs => {
                "MkDocs with Material theme, hosted on ReadTheDocs. \
                 Recommended for Python projects."
            }
            Self::Sphinx => {
                "Sphinx with autodoc for Python API documentation. \
                 Best for Python libraries."
            }
            Self::MdBook => {
                "mdBook — book-style guides for Rust applications. \
                 Use rustdoc for API reference."
            }
            Self::Rustdoc => {
                "cargo doc with docs.rs auto-publish. \
                 Recommended for Rust library crates."
            }
            Self::Doxygen => {
                "Doxygen with Graphviz call graphs. Standard for C, C++, \
                 and HDL (VHDL/Verilog) projects."
            }
            Self::JsDoc => {
                "JSDoc for JavaScript API documentation. \
                 Use TypeDoc for TypeScript."
            }
            Self::TypeDoc => {
                "TypeDoc with TSDoc comments for TypeScript API docs. \
                 Type-aware reflection."
            }
            Self::Javadoc => {
                "Javadoc (Java) or Dokka (Kotlin/mixed) via Gradle. \
                 Standard JVM API docs."
            }
            Self::OpenApi => {
                "OpenAPI 3.1 YAML spec with Swagger UI / Redocly validation. \
                 For REST API projects."
            }
            Self::None => {
                "No documentation system. Not recommended — every project \
                 should have at least a MANUAL.md (H23)."
            }
        }
    }
}

/// Where generated docs are deployed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocDeployTarget {
    /// No automated deployment — build locally only.
    #[default]
    LocalOnly,
    /// ReadTheDocs (specsmith default — Python/MkDocs/Sphinx projects).
    ReadTheDocs,
    /// GitHub Pages via GitHub Actions.
    GitHubPages,
    /// docs.rs (automatic for Rust crates published to crates.io).
    DocsRs,
}

impl DocDeployTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::LocalOnly => "Local only",
            Self::ReadTheDocs => "ReadTheDocs",
            Self::GitHubPages => "GitHub Pages",
            Self::DocsRs => "docs.rs (Rust crates)",
        }
    }
}

// ---------------------------------------------------------------------------
// State (persisted)
// ---------------------------------------------------------------------------

pub struct DocSystemState {
    pub doc_system: DocSystem,
    pub deploy_target: DocDeployTarget,
}

impl DocSystemState {
    fn system_file_path() -> std::path::PathBuf {
        warp_core::paths::data_dir().join("kairos_doc_system")
    }

    fn deploy_file_path() -> std::path::PathBuf {
        warp_core::paths::data_dir().join("kairos_doc_deploy")
    }

    pub fn load() -> Self {
        let doc_system = std::fs::read_to_string(Self::system_file_path())
            .ok()
            .as_deref()
            .map(DocSystem::from_str)
            .unwrap_or_default();
        let deploy_target = std::fs::read_to_string(Self::deploy_file_path())
            .ok()
            .map(|s| match s.trim() {
                "rtd" => DocDeployTarget::ReadTheDocs,
                "github-pages" => DocDeployTarget::GitHubPages,
                "docs-rs" => DocDeployTarget::DocsRs,
                _ => DocDeployTarget::LocalOnly,
            })
            .unwrap_or_default();
        Self {
            doc_system,
            deploy_target,
        }
    }

    pub fn save_system(doc_system: DocSystem) {
        let path = Self::system_file_path();
        if let Some(p) = path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let _ = std::fs::write(&path, doc_system.as_file_str());
    }

    pub fn save_deploy(target: DocDeployTarget) {
        let path = Self::deploy_file_path();
        if let Some(p) = path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let s = match target {
            DocDeployTarget::ReadTheDocs => "rtd",
            DocDeployTarget::GitHubPages => "github-pages",
            DocDeployTarget::DocsRs => "docs-rs",
            DocDeployTarget::LocalOnly => "local",
        };
        let _ = std::fs::write(&path, s);
    }
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum DocsPageAction {
    SetDocSystem(DocSystem),
    SetDeployTarget(DocDeployTarget),
    OpenSkillDocs,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct DocsPageView {
    page: PageType<Self>,
    state: DocSystemState,
    // button handles for pill selectors
    btn_user_manual: MouseStateHandle,
    btn_mkdocs: MouseStateHandle,
    btn_sphinx: MouseStateHandle,
    btn_mdbook: MouseStateHandle,
    btn_rustdoc: MouseStateHandle,
    btn_doxygen: MouseStateHandle,
    btn_jsdoc: MouseStateHandle,
    btn_typedoc: MouseStateHandle,
    btn_javadoc: MouseStateHandle,
    btn_openapi: MouseStateHandle,
    btn_none: MouseStateHandle,
    btn_local: MouseStateHandle,
    btn_rtd: MouseStateHandle,
    btn_ghpages: MouseStateHandle,
    btn_docsrs: MouseStateHandle,
    btn_open_skill: MouseStateHandle,
}

impl DocsPageView {
    pub fn new(_ctx: &mut ViewContext<DocsPageView>) -> Self {
        let state = DocSystemState::load();
        DocsPageView {
            page: PageType::new_monolith(DocsPageWidget::default(), None, false),
            state,
            btn_user_manual: MouseStateHandle::default(),
            btn_mkdocs: MouseStateHandle::default(),
            btn_sphinx: MouseStateHandle::default(),
            btn_mdbook: MouseStateHandle::default(),
            btn_rustdoc: MouseStateHandle::default(),
            btn_doxygen: MouseStateHandle::default(),
            btn_jsdoc: MouseStateHandle::default(),
            btn_typedoc: MouseStateHandle::default(),
            btn_javadoc: MouseStateHandle::default(),
            btn_openapi: MouseStateHandle::default(),
            btn_none: MouseStateHandle::default(),
            btn_local: MouseStateHandle::default(),
            btn_rtd: MouseStateHandle::default(),
            btn_ghpages: MouseStateHandle::default(),
            btn_docsrs: MouseStateHandle::default(),
            btn_open_skill: MouseStateHandle::default(),
        }
    }
}

impl Entity for DocsPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for DocsPageView {
    type Action = DocsPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            DocsPageAction::SetDocSystem(sys) => {
                self.state.doc_system = *sys;
                DocSystemState::save_system(*sys);
                ctx.notify();
            }
            DocsPageAction::SetDeployTarget(target) => {
                self.state.deploy_target = *target;
                DocSystemState::save_deploy(*target);
                ctx.notify();
            }
            DocsPageAction::OpenSkillDocs => {
                let url = "https://specsmith.readthedocs.io/en/latest/skills-index.html";
                ctx.open_url(url);
            }
        }
    }
}

impl View for DocsPageView {
    fn ui_name() -> &'static str {
        "DocsPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

#[derive(Default)]
struct DocsPageWidget;

impl SettingsWidget for DocsPageWidget {
    type View = DocsPageView;

    fn search_terms(&self) -> &str {
        "documentation docs mkdocs sphinx mdbook rustdoc doxygen jsdoc typedoc \
         javadoc openapi swagger manual rtd readthedocs"
    }

    fn render(
        &self,
        view: &DocsPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let ui = appearance.ui_builder();
        let current = view.state.doc_system;
        let current_deploy = view.state.deploy_target;

        // ── Documentation system selector ─────────────────────────────────────

        let system_header = build_sub_header(appearance, "Documentation System", None).finish();

        let description_text = ui
            .span(current.description().to_string())
            .build()
            .with_margin_bottom(8.)
            .finish();

        // Build a pill button for each doc system.
        let make_pill =
            |label: &str, system: DocSystem, selected: bool, handle: MouseStateHandle| {
                let variant = if selected {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                };
                ui.button(label, variant)
                    .build()
                    .mouse_state(handle)
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(DocsPageAction::SetDocSystem(system));
                    })
                    .with_margin_right(6.)
                    .with_margin_bottom(6.)
                    .finish()
            };

        let pills_row = Flex::row()
            .with_children([
                make_pill(
                    DocSystem::UserManual.label(),
                    DocSystem::UserManual,
                    current == DocSystem::UserManual,
                    view.btn_user_manual.clone(),
                ),
                make_pill(
                    DocSystem::MkDocs.label(),
                    DocSystem::MkDocs,
                    current == DocSystem::MkDocs,
                    view.btn_mkdocs.clone(),
                ),
                make_pill(
                    DocSystem::Sphinx.label(),
                    DocSystem::Sphinx,
                    current == DocSystem::Sphinx,
                    view.btn_sphinx.clone(),
                ),
                make_pill(
                    DocSystem::MdBook.label(),
                    DocSystem::MdBook,
                    current == DocSystem::MdBook,
                    view.btn_mdbook.clone(),
                ),
                make_pill(
                    DocSystem::Rustdoc.label(),
                    DocSystem::Rustdoc,
                    current == DocSystem::Rustdoc,
                    view.btn_rustdoc.clone(),
                ),
                make_pill(
                    DocSystem::Doxygen.label(),
                    DocSystem::Doxygen,
                    current == DocSystem::Doxygen,
                    view.btn_doxygen.clone(),
                ),
                make_pill(
                    DocSystem::JsDoc.label(),
                    DocSystem::JsDoc,
                    current == DocSystem::JsDoc,
                    view.btn_jsdoc.clone(),
                ),
                make_pill(
                    DocSystem::TypeDoc.label(),
                    DocSystem::TypeDoc,
                    current == DocSystem::TypeDoc,
                    view.btn_typedoc.clone(),
                ),
                make_pill(
                    DocSystem::Javadoc.label(),
                    DocSystem::Javadoc,
                    current == DocSystem::Javadoc,
                    view.btn_javadoc.clone(),
                ),
                make_pill(
                    DocSystem::OpenApi.label(),
                    DocSystem::OpenApi,
                    current == DocSystem::OpenApi,
                    view.btn_openapi.clone(),
                ),
                make_pill(
                    DocSystem::None.label(),
                    DocSystem::None,
                    current == DocSystem::None,
                    view.btn_none.clone(),
                ),
            ])
            .finish();

        // ── Skill hint ────────────────────────────────────────────────────────

        let skill_hint = if let Some(slug) = current.skill_slug() {
            let hint_text = format!(
                "specsmith skill: `{slug}` — run `specsmith skill activate {slug}` \
                 to inject the documentation skill into your project."
            );
            ui.span(hint_text)
                .build()
                .with_margin_top(8.)
                .with_margin_bottom(8.)
                .finish()
        } else {
            ui.span(
                "No documentation skill configured. \
                 Consider selecting User Manual as a minimum."
                    .to_string(),
            )
            .build()
            .with_margin_top(8.)
            .finish()
        };

        let open_skill_button = ui
            .button("Browse Skills Index", ButtonVariant::Secondary)
            .build()
            .mouse_state(view.btn_open_skill.clone())
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(DocsPageAction::OpenSkillDocs);
            })
            .with_margin_top(4.)
            .finish();

        // ── Deploy target ─────────────────────────────────────────────────────

        let sep = render_separator(appearance);
        let deploy_header =
            build_sub_header(appearance, "Documentation Deploy Target", None).finish();

        let make_deploy_pill =
            |label: &str, target: DocDeployTarget, selected: bool, handle: MouseStateHandle| {
                let variant = if selected {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                };
                ui.button(label, variant)
                    .build()
                    .mouse_state(handle)
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(DocsPageAction::SetDeployTarget(target));
                    })
                    .with_margin_right(6.)
                    .finish()
            };

        let deploy_row = Flex::row()
            .with_children([
                make_deploy_pill(
                    DocDeployTarget::LocalOnly.label(),
                    DocDeployTarget::LocalOnly,
                    current_deploy == DocDeployTarget::LocalOnly,
                    view.btn_local.clone(),
                ),
                make_deploy_pill(
                    DocDeployTarget::ReadTheDocs.label(),
                    DocDeployTarget::ReadTheDocs,
                    current_deploy == DocDeployTarget::ReadTheDocs,
                    view.btn_rtd.clone(),
                ),
                make_deploy_pill(
                    DocDeployTarget::GitHubPages.label(),
                    DocDeployTarget::GitHubPages,
                    current_deploy == DocDeployTarget::GitHubPages,
                    view.btn_ghpages.clone(),
                ),
                make_deploy_pill(
                    DocDeployTarget::DocsRs.label(),
                    DocDeployTarget::DocsRs,
                    current_deploy == DocDeployTarget::DocsRs,
                    view.btn_docsrs.clone(),
                ),
            ])
            .finish();

        // ── Governance note ───────────────────────────────────────────────────

        let gov_note = ui
            .span(
                "H23 Documentation Lifecycle Gate — arch → req → tests → docs. \
                 H24 Skills Documentation Required — every skill must appear in \
                 docs/site/skills-index.md."
                    .to_string(),
            )
            .build()
            .with_margin_top(12.)
            .finish();

        // ── Assemble ──────────────────────────────────────────────────────────

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(
                Container::new(system_header)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(description_text)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(pills_row)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(skill_hint)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(open_skill_button)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(sep)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(deploy_header)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(deploy_row)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .with_child(
                Container::new(gov_note)
                    .with_padding_horizontal(HEADER_PADDING)
                    .finish(),
            )
            .finish()
    }
}

// ---------------------------------------------------------------------------
// SettingsPageMeta
// ---------------------------------------------------------------------------

impl SettingsPageMeta for DocsPageView {
    fn section() -> SettingsSection {
        SettingsSection::DocSystem
    }

    fn match_data() -> MatchData {
        MatchData {
            additional_match_data: Some("documentation docs mkdocs rtd readthedocs"),
        }
    }
}
