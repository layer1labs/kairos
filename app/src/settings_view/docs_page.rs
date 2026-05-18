//! Documentation settings page — choose a documentation system for the current project.
//!
//! H23 Documentation Lifecycle Gate: arch → req → tests → docs.
//! H24 Skills Documentation Required: every skill appears in skills-index.md.
//!
//! Default: UserManual (MANUAL.md) — never None.

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
// Documentation system model
// ---------------------------------------------------------------------------

/// Which documentation system is configured for this project.
///
/// Default: [`DocSystem::UserManual`] — a single MANUAL.md file.
/// `None` is intentionally last and not the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocSystem {
    /// MANUAL.md — single-file bare-minimum docs.  **Suggested default.**
    UserManual,
    /// MkDocs with Material theme + ReadTheDocs.
    MkDocs,
    /// Sphinx with autodoc + MyST.
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
    fn default() -> Self {
        Self::UserManual
    }
}

impl DocSystem {
    pub fn label(self) -> &'static str {
        match self {
            Self::UserManual => "User Manual (MANUAL.md)",
            Self::MkDocs => "MkDocs / RTD",
            Self::Sphinx => "Sphinx / MyST",
            Self::MdBook => "mdBook (Rust)",
            Self::Rustdoc => "rustdoc",
            Self::Doxygen => "Doxygen (C/C++/VHDL)",
            Self::JsDoc => "JSDoc",
            Self::TypeDoc => "TypeDoc (TypeScript)",
            Self::Javadoc => "Javadoc / Dokka",
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

    pub fn description(self) -> &'static str {
        match self {
            Self::UserManual => "MANUAL.md at the project root \u{2014} bare minimum, always safe.",
            Self::MkDocs => "MkDocs + Material, hosted on ReadTheDocs. Best for Python.",
            Self::Sphinx => "Sphinx + autodoc. Best for Python API documentation.",
            Self::MdBook => "mdBook \u{2014} book-style guides for Rust applications.",
            Self::Rustdoc => "cargo doc + docs.rs. Best for Rust library crates.",
            Self::Doxygen => "Doxygen + Graphviz. Standard for C, C++, and HDL projects.",
            Self::JsDoc => "JSDoc for JavaScript API docs. Use TypeDoc for TypeScript.",
            Self::TypeDoc => "TypeDoc with TSDoc \u{2014} type-aware TypeScript API docs.",
            Self::Javadoc => "Javadoc (Java) or Dokka (Kotlin/mixed) via Gradle.",
            Self::OpenApi => "OpenAPI 3.1 YAML spec + Swagger UI / Redocly. For REST APIs.",
            Self::None => {
                "No documentation system. Not recommended \u{2014} use at least MANUAL.md."
            }
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
            _ => Self::UserManual,
        }
    }
}

// ---------------------------------------------------------------------------
// State (persisted)
// ---------------------------------------------------------------------------

pub struct DocSystemState {
    pub doc_system: DocSystem,
}

impl DocSystemState {
    fn file_path() -> std::path::PathBuf {
        warp_core::paths::data_dir().join("kairos_doc_system")
    }

    pub fn load() -> Self {
        let doc_system = std::fs::read_to_string(Self::file_path())
            .ok()
            .as_deref()
            .map(DocSystem::from_str)
            .unwrap_or_default();
        Self { doc_system }
    }

    pub fn save(doc_system: DocSystem) {
        let path = Self::file_path();
        if let Some(p) = path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let _ = std::fs::write(&path, doc_system.as_file_str());
    }
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum DocsPageAction {
    SetDocSystem(DocSystem),
    OpenSkillDocs,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct DocsPageView {
    page: PageType<Self>,
    state: DocSystemState,
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
    btn_skills_link: MouseStateHandle,
}

impl DocsPageView {
    pub fn new(_ctx: &mut ViewContext<DocsPageView>) -> Self {
        DocsPageView {
            page: PageType::new_monolith(DocsPageWidget::default(), None, false),
            state: DocSystemState::load(),
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
            btn_skills_link: MouseStateHandle::default(),
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
                DocSystemState::save(*sys);
                ctx.notify();
            }
            DocsPageAction::OpenSkillDocs => {
                ctx.open_url("https://specsmith.readthedocs.io/en/latest/skills-index.html");
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

impl DocsPageWidget {
    fn doc_btn(
        label: &str,
        system: DocSystem,
        selected: bool,
        mouse_state: MouseStateHandle,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
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
                ctx.dispatch_typed_action(DocsPageAction::SetDocSystem(system));
            })
            .finish()
    }
}

impl SettingsWidget for DocsPageWidget {
    type View = DocsPageView;

    fn search_terms(&self) -> &str {
        "documentation docs mkdocs sphinx mdbook rustdoc doxygen jsdoc typedoc \
         javadoc openapi swagger manual rtd readthedocs H23 H24"
    }

    fn render(
        &self,
        view: &DocsPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let cur = view.state.doc_system;

        let header = build_sub_header(appearance, "Documentation System", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let desc = Text::new(
            cur.description().to_string(),
            appearance.ui_font_family(),
            12.,
        )
        .with_color(dim.into())
        .soft_wrap(true)
        .finish();

        // Pill row 1: Python/generic/API
        let row1 = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(Self::doc_btn(
                    DocSystem::UserManual.label(),
                    DocSystem::UserManual,
                    cur == DocSystem::UserManual,
                    view.btn_user_manual.clone(),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::doc_btn(
                    DocSystem::MkDocs.label(),
                    DocSystem::MkDocs,
                    cur == DocSystem::MkDocs,
                    view.btn_mkdocs.clone(),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::doc_btn(
                    DocSystem::Sphinx.label(),
                    DocSystem::Sphinx,
                    cur == DocSystem::Sphinx,
                    view.btn_sphinx.clone(),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(Self::doc_btn(
                DocSystem::OpenApi.label(),
                DocSystem::OpenApi,
                cur == DocSystem::OpenApi,
                view.btn_openapi.clone(),
                appearance,
            ))
            .finish();

        // Pill row 2: Rust/C++/JS
        let row2 = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(Self::doc_btn(
                    DocSystem::MdBook.label(),
                    DocSystem::MdBook,
                    cur == DocSystem::MdBook,
                    view.btn_mdbook.clone(),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::doc_btn(
                    DocSystem::Rustdoc.label(),
                    DocSystem::Rustdoc,
                    cur == DocSystem::Rustdoc,
                    view.btn_rustdoc.clone(),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::doc_btn(
                    DocSystem::Doxygen.label(),
                    DocSystem::Doxygen,
                    cur == DocSystem::Doxygen,
                    view.btn_doxygen.clone(),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::doc_btn(
                    DocSystem::JsDoc.label(),
                    DocSystem::JsDoc,
                    cur == DocSystem::JsDoc,
                    view.btn_jsdoc.clone(),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::doc_btn(
                    DocSystem::TypeDoc.label(),
                    DocSystem::TypeDoc,
                    cur == DocSystem::TypeDoc,
                    view.btn_typedoc.clone(),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(
                Container::new(Self::doc_btn(
                    DocSystem::Javadoc.label(),
                    DocSystem::Javadoc,
                    cur == DocSystem::Javadoc,
                    view.btn_javadoc.clone(),
                    appearance,
                ))
                .with_margin_right(6.)
                .finish(),
            )
            .with_child(Self::doc_btn(
                DocSystem::None.label(),
                DocSystem::None,
                cur == DocSystem::None,
                view.btn_none.clone(),
                appearance,
            ))
            .finish();

        let skill_hint_text = if let Some(slug) = cur.skill_slug() {
            format!(
                "specsmith skill: {slug}  \u{2014}  run `specsmith skill activate {slug}` \
                 to inject the documentation skill into your project."
            )
        } else {
            "No skill configured. Consider selecting User Manual as a minimum.".to_owned()
        };

        let skill_hint = Text::new(skill_hint_text, appearance.ui_font_family(), 12.)
            .with_color(dim.into())
            .soft_wrap(true)
            .finish();

        let skills_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.btn_skills_link.clone())
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label("Browse Skills Index \u{2197}".to_string())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(DocsPageAction::OpenSkillDocs);
            })
            .finish();

        let gov_note = Text::new(
            "H23: arch \u{2192} req \u{2192} tests \u{2192} docs  \u{2014}  \
             H24: every skill must appear in docs/site/skills-index.md."
                .to_string(),
            appearance.ui_font_family(),
            11.,
        )
        .with_color(dim.into())
        .soft_wrap(true)
        .finish();

        Container::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(header)
                .with_child(Container::new(desc).with_margin_bottom(12.).finish())
                .with_child(Container::new(row1).with_margin_bottom(6.).finish())
                .with_child(Container::new(row2).with_margin_bottom(12.).finish())
                .with_child(render_separator(appearance))
                .with_child(Container::new(skill_hint).with_margin_bottom(8.).finish())
                .with_child(Container::new(skills_btn).with_margin_bottom(12.).finish())
                .with_child(render_separator(appearance))
                .with_child(gov_note)
                .finish(),
        )
        .with_uniform_padding(28.)
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

impl From<warpui::ViewHandle<DocsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: warpui::ViewHandle<DocsPageView>) -> Self {
        SettingsPageViewHandle::DocSystem(view_handle)
    }
}
