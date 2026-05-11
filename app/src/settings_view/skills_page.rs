//! Skills management page — browse, build, activate/deactivate, test, delete AI agent skills.
//!
//! Sections:
//!  1. Skills Builder — natural-language description → specsmith skills build
//!  2. Skills list    — all skills with status, activate/deactivate, test, delete buttons
//!  3. Refresh        — re-fetch skills list

use super::{
    settings_page::{
        build_sub_header, render_separator, MatchData, PageType, SettingsPageEvent,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use crate::themes::theme::Fill;
use std::path::PathBuf;
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
    tags: Vec<String>,
    tools_used: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum SkillsStatus {
    Unknown,
    Loading,
    Loaded(Vec<SkillEntry>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
enum BuildStatus {
    #[default]
    Idle,
    Building,
    Done(String),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
enum SkillOpStatus {
    #[default]
    Idle,
    Running(String),
    Done(String),
    Error(String),
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SkillsPageAction {
    Refresh,
    /// Build a new skill from the current builder_description.
    Build,
    /// Toggle the builder description input (simulated — updates a stored string).
    SetBuilderDescription(String),
    /// Activate a skill by ID.
    Activate(String),
    /// Deactivate a skill by ID.
    Deactivate(String),
    /// Dry-run test a skill by ID.
    Test(String),
    /// Delete a skill by ID.
    Delete(String),
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct SkillsPageView {
    page: PageType<Self>,
    status: SkillsStatus,
    build_status: BuildStatus,
    skill_op_status: SkillOpStatus,
    builder_description: String,
    project_dir: Option<PathBuf>,
    refresh_button: MouseStateHandle,
    build_button: MouseStateHandle,
}

impl SkillsPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let initial_dir = std::env::current_dir().ok();
        let mut view = SkillsPageView {
            page: PageType::new_monolith(SkillsPageWidget::default(), None, false),
            status: SkillsStatus::Unknown,
            build_status: BuildStatus::Idle,
            skill_op_status: SkillOpStatus::Idle,
            builder_description: String::new(),
            project_dir: initial_dir,
            refresh_button: MouseStateHandle::default(),
            build_button: MouseStateHandle::default(),
        };
        view.fetch_skills(ctx);
        view
    }

    fn run_skills_cmd(
        &mut self,
        label: impl Into<String>,
        cmd_args: Vec<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        let label = label.into();
        self.skill_op_status = SkillOpStatus::Running(label);
        ctx.notify();
        let project_dir = self.project_dir.clone();
        ctx.spawn(
            async move {
                let run = |prog: &str, args: &[&str]| -> Result<String, String> {
                    let mut c = std::process::Command::new(prog);
                    c.args(args);
                    if let Some(dir) = &project_dir {
                        c.current_dir(dir);
                    }
                    c.env("SPECSMITH_NO_AUTO_UPDATE", "1");
                    c.env("SPECSMITH_PYPI_CHECKED", "1");
                    c.output()
                        .map(|o| {
                            format!(
                                "{}{}",
                                String::from_utf8_lossy(&o.stdout),
                                String::from_utf8_lossy(&o.stderr)
                            )
                        })
                        .map_err(|e| e.to_string())
                };
                let args_refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
                let mut py_args = vec!["-m", "specsmith"];
                py_args.extend_from_slice(&args_refs);
                run("py", &py_args)
                    .or_else(|_| run("specsmith", &args_refs))
                    .map_err(|e| format!("specsmith not found: {e}"))
            },
            |me, result: Result<String, String>, ctx| {
                me.skill_op_status = match result {
                    Ok(output) => {
                        SkillOpStatus::Done(output.lines().take(10).collect::<Vec<_>>().join("\n"))
                    }
                    Err(e) => SkillOpStatus::Error(e.chars().take(200).collect()),
                };
                // Re-fetch skills after activate/deactivate/delete
                me.fetch_skills(ctx);
                ctx.notify();
            },
        );
    }

    fn build_skill(&mut self, ctx: &mut ViewContext<Self>) {
        let desc = self.builder_description.trim().to_owned();
        if desc.is_empty() {
            return;
        }
        self.build_status = BuildStatus::Building;
        ctx.notify();
        let project_dir = self.project_dir.clone();
        ctx.spawn(
            async move {
                let run = |prog: &str, args: &[&str]| -> Result<String, String> {
                    let mut c = std::process::Command::new(prog);
                    c.args(args);
                    if let Some(dir) = &project_dir {
                        c.current_dir(dir);
                    }
                    c.env("SPECSMITH_NO_AUTO_UPDATE", "1");
                    c.env("SPECSMITH_PYPI_CHECKED", "1");
                    c.output()
                        .map(|o| {
                            format!(
                                "{}{}",
                                String::from_utf8_lossy(&o.stdout),
                                String::from_utf8_lossy(&o.stderr)
                            )
                        })
                        .map_err(|e| e.to_string())
                };
                let py_args = ["-m", "specsmith", "skills", "build", &desc];
                run("py", &py_args)
                    .or_else(|_| run("specsmith", &["skills", "build", &desc]))
                    .map_err(|e| format!("specsmith not found: {e}"))
            },
            |me, result: Result<String, String>, ctx| {
                me.build_status = match result {
                    Ok(output) => {
                        BuildStatus::Done(output.lines().take(5).collect::<Vec<_>>().join("\n"))
                    }
                    Err(e) => BuildStatus::Error(e.chars().take(200).collect()),
                };
                me.fetch_skills(ctx);
                ctx.notify();
            },
        );
    }

    fn fetch_skills(&mut self, ctx: &mut ViewContext<Self>) {
        self.status = SkillsStatus::Loading;
        ctx.notify();
        let project_dir = self.project_dir.clone();
        ctx.spawn(
            async move {
                let mut c = std::process::Command::new("py");
                let args = ["-m", "specsmith", "skills", "list", "--json"];
                if let Some(dir) = &project_dir {
                    c.current_dir(dir);
                }
                c.args(&args);
                c.env("SPECSMITH_NO_AUTO_UPDATE", "1");
                c.env("SPECSMITH_PYPI_CHECKED", "1");
                let out = c.output().map_err(|e| e.to_string())?;
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                let val: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
                let skills: Vec<SkillEntry> = val
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
                                    tags: v
                                        .get("tags")
                                        .and_then(|t| t.as_array())
                                        .map(|a| {
                                            a.iter()
                                                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                                                .collect()
                                        })
                                        .unwrap_or_default(),
                                    tools_used: v
                                        .get("tools_used")
                                        .and_then(|t| t.as_array())
                                        .map(|a| {
                                            a.iter()
                                                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                                                .collect()
                                        })
                                        .unwrap_or_default(),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(skills)
            },
            |me, result: Result<Vec<SkillEntry>, String>, ctx| {
                me.status = match result {
                    Ok(skills) => SkillsStatus::Loaded(skills),
                    Err(e) => SkillsStatus::Error(e),
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
            SkillsPageAction::Build => self.build_skill(ctx),
            SkillsPageAction::SetBuilderDescription(desc) => {
                self.builder_description = desc.clone();
                ctx.notify();
            }
            SkillsPageAction::Activate(id) => {
                let label = format!("activate {id}");
                let args = vec!["skills".to_owned(), "activate".to_owned(), id.clone()];
                self.run_skills_cmd(label, args, ctx);
            }
            SkillsPageAction::Deactivate(id) => {
                let label = format!("deactivate {id}");
                let args = vec!["skills".to_owned(), "deactivate".to_owned(), id.clone()];
                self.run_skills_cmd(label, args, ctx);
            }
            SkillsPageAction::Test(id) => {
                let label = format!("test {id}");
                let args = vec!["skills".to_owned(), "test".to_owned(), id.clone()];
                self.run_skills_cmd(label, args, ctx);
            }
            SkillsPageAction::Delete(id) => {
                let label = format!("delete {id}");
                let args = vec![
                    "skills".to_owned(),
                    "delete".to_owned(),
                    "--yes".to_owned(),
                    id.clone(),
                ];
                self.run_skills_cmd(label, args, ctx);
            }
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

    fn small_btn(
        label: impl Into<String>,
        variant: ButtonVariant,
        ms: MouseStateHandle,
        action: SkillsPageAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        appearance
            .ui_builder()
            .button(variant, ms)
            .with_style(UiComponentStyles {
                font_size: Some(10.),
                padding: Some(Coords::uniform(4.)),
                ..Default::default()
            })
            .with_centered_text_label(label.into())
            .build()
            .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
            .finish()
    }
}

impl SettingsWidget for SkillsPageWidget {
    type View = SkillsPageView;

    fn search_terms(&self) -> &str {
        "skills agent build activate deactivate test ai automation epistemic tools"
    }

    fn render(
        &self,
        view: &SkillsPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color();
        let accent: Fill = theme.accent().into_solid().into();

        // ── Section 1: Skills Builder ─────────────────────────────────────
        let builder_header = build_sub_header(appearance, "Skills Builder", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        // Description hint (no interactive text input in this renderer pass —
        // shows the current description and a placeholder hint)
        let desc_text = if view.builder_description.is_empty() {
            "Describe a skill to build, e.g. \u{201c}Summarise Python files into bullet points\u{201d}".to_string()
        } else {
            view.builder_description.clone()
        };
        let desc_color = if view.builder_description.is_empty() {
            dim
        } else {
            active
        };

        let build_output: Option<Box<dyn Element>> = match &view.build_status {
            BuildStatus::Idle => None,
            BuildStatus::Building => Some(
                Text::new(
                    "Building skill\u{2026}".to_string(),
                    appearance.ui_font_family(),
                    11.,
                )
                .with_color(dim.into())
                .finish(),
            ),
            BuildStatus::Done(output) => Some(
                Text::new(output.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(active.into())
                    .soft_wrap(true)
                    .finish(),
            ),
            BuildStatus::Error(msg) => Some(
                Text::new(msg.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(theme.ui_error_color().into())
                    .soft_wrap(true)
                    .finish(),
            ),
        };

        let build_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Accent, view.build_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label("Build Skill".to_string())
            .build()
            .on_click(|ctx, _, _| ctx.dispatch_typed_action(SkillsPageAction::Build))
            .finish();

        let mut builder_col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Container::new(
                    Text::new(desc_text, appearance.ui_font_family(), 12.)
                        .with_color(desc_color.into())
                        .soft_wrap(true)
                        .finish(),
                )
                .with_margin_bottom(8.)
                .finish(),
            )
            .with_child(
                Container::new(
                    Text::new(
                        "Run: specsmith skills build \"<description>\"".to_string(),
                        appearance.monospace_font_family(),
                        11.,
                    )
                    .with_color(dim.into())
                    .finish(),
                )
                .with_margin_bottom(10.)
                .finish(),
            )
            .with_child(build_btn);

        if let Some(out) = build_output {
            builder_col.add_child(Container::new(out).with_margin_top(8.).finish());
        }

        let builder_card = Self::card(builder_col.finish(), appearance);

        // ── Section 2: Skills list ────────────────────────────────────────
        let skills_header = build_sub_header(appearance, "AI Agent Skills", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        // Skill op status
        let op_output: Option<Box<dyn Element>> = match &view.skill_op_status {
            SkillOpStatus::Idle => None,
            SkillOpStatus::Running(label) => Some(
                Text::new(
                    format!("Running {label}\u{2026}"),
                    appearance.ui_font_family(),
                    11.,
                )
                .with_color(dim.into())
                .finish(),
            ),
            SkillOpStatus::Done(output) => Some(
                Text::new(output.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(active.into())
                    .soft_wrap(true)
                    .finish(),
            ),
            SkillOpStatus::Error(msg) => Some(
                Text::new(msg.clone(), appearance.monospace_font_family(), 10.)
                    .with_color(theme.ui_error_color().into())
                    .soft_wrap(true)
                    .finish(),
            ),
        };

        let skills_content_card = match &view.status {
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
                    .finish(),
                appearance,
            ),
            SkillsStatus::Loaded(skills) if skills.is_empty() => Self::card(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(
                        Text::new(
                            "No skills configured.".to_string(),
                            appearance.ui_font_family(),
                            13.,
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new(
                                "Use the Skills Builder above to create your first skill."
                                    .to_string(),
                                appearance.ui_font_family(),
                                12.,
                            )
                            .with_color(dim.into())
                            .soft_wrap(true)
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
                    let tags_str = if skill.tags.is_empty() {
                        String::new()
                    } else {
                        format!("  [{}]", skill.tags.join(", "))
                    };
                    let tools_str = if skill.tools_used.is_empty() {
                        String::new()
                    } else {
                        format!("  tools: {}", skill.tools_used.join(", "))
                    };

                    // Per-skill action buttons — use inline MouseStateHandles
                    // (accepted pattern for list items where count is not known at compile time)
                    let act_ms = MouseStateHandle::default();
                    let test_ms = MouseStateHandle::default();
                    let del_ms = MouseStateHandle::default();
                    let skill_id = skill.id.clone();
                    let skill_id2 = skill.id.clone();
                    let skill_id3 = skill.id.clone();

                    let (toggle_label, toggle_action) = if skill.active {
                        ("Deactivate", SkillsPageAction::Deactivate(skill_id))
                    } else {
                        ("Activate", SkillsPageAction::Activate(skill_id))
                    };

                    let row = Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .with_child(
                            Flex::row()
                                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                .with_child(
                                    Container::new(
                                        Text::new_inline(
                                            badge_text.to_string(),
                                            appearance.ui_font_family(),
                                            11.,
                                        )
                                        .with_color(badge_color.into())
                                        .finish(),
                                    )
                                    .with_margin_right(10.)
                                    .finish(),
                                )
                                .with_child(
                                    Text::new_inline(
                                        skill.name.clone(),
                                        appearance.ui_font_family(),
                                        13.,
                                    )
                                    .with_color(active.into())
                                    .finish(),
                                )
                                .with_child(
                                    Container::new(
                                        Text::new_inline(
                                            tags_str,
                                            appearance.ui_font_family(),
                                            10.,
                                        )
                                        .with_color(dim.into())
                                        .finish(),
                                    )
                                    .with_margin_left(6.)
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
                        .with_child(
                            Container::new(
                                Text::new(tools_str, appearance.monospace_font_family(), 10.)
                                    .with_color(dim.into())
                                    .finish(),
                            )
                            .with_margin_top(2.)
                            .finish(),
                        )
                        .with_child(
                            Container::new(
                                Flex::row()
                                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                    .with_child(Self::small_btn(
                                        toggle_label,
                                        if skill.active {
                                            ButtonVariant::Secondary
                                        } else {
                                            ButtonVariant::Accent
                                        },
                                        act_ms,
                                        toggle_action,
                                        appearance,
                                    ))
                                    .with_child(
                                        Container::new(Self::small_btn(
                                            "Test",
                                            ButtonVariant::Secondary,
                                            test_ms,
                                            SkillsPageAction::Test(skill_id2),
                                            appearance,
                                        ))
                                        .with_margin_left(5.)
                                        .finish(),
                                    )
                                    .with_child(
                                        Container::new(Self::small_btn(
                                            "Delete",
                                            ButtonVariant::Secondary,
                                            del_ms,
                                            SkillsPageAction::Delete(skill_id3),
                                            appearance,
                                        ))
                                        .with_margin_left(5.)
                                        .finish(),
                                    )
                                    .finish(),
                            )
                            .with_margin_top(6.)
                            .finish(),
                        )
                        .finish();

                    if i > 0 {
                        col.add_child(
                            Container::new(row)
                                .with_margin_top(14.)
                                .with_padding_top(10.)
                                .finish(),
                        );
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

                if let Some(out) = op_output {
                    col.add_child(Container::new(out).with_margin_top(10.).finish());
                }

                Self::card(col.finish(), appearance)
            }
        };

        // ── Refresh button ────────────────────────────────────────────────
        let refresh_btn = appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, view.refresh_button.clone())
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                padding: Some(Coords::uniform(6.)),
                ..Default::default()
            })
            .with_centered_text_label("Refresh".to_string())
            .build()
            .on_click(|ctx, _, _| ctx.dispatch_typed_action(SkillsPageAction::Refresh))
            .finish();

        // ── Assemble ──────────────────────────────────────────────────────
        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(builder_header)
                    .with_child(builder_card)
                    .with_child(render_separator(appearance))
                    .with_child(skills_header)
                    .with_child(skills_content_card)
                    .with_child(Container::new(refresh_btn).with_margin_top(8.).finish())
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
