//! Agent Global Defaults settings page.
//!
//! Sets the model/provider used when opening a brand-new agent session that has
//! no previous project-level override.  Projects may override these defaults on
//! a per-directory basis; this page also controls whether that is allowed.
//!
//! Persistence: ~/.specsmith/agent_defaults.json

use super::{
    settings_page::{
        build_sub_header, render_separator, MatchData, PageType, SettingsPageEvent,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use crate::themes::theme::Fill;
use crate::view_components::{SubmittableTextInput, SubmittableTextInputEvent};
use warpui::{
    elements::{
        ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element, Expanded,
        Flex, Hoverable, MouseStateHandle, ParentElement, Radius, Text,
    },
    fonts::{Properties, Weight},
    platform::Cursor,
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

// ── Persistence ───────────────────────────────────────────────────────────────

fn defaults_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".specsmith").join("agent_defaults.json"))
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentDefaults {
    /// Model ID used for new agents with no project override.
    pub default_model_id: String,
    /// Provider ID (openai, anthropic, google, etc.).
    pub default_provider_id: String,
    /// If true, projects can override these defaults via scaffold.yml / .specsmith/.
    pub allow_project_override: bool,
}

fn load_defaults() -> AgentDefaults {
    let path = match defaults_path() {
        Some(p) => p,
        None => return AgentDefaults::default(),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return AgentDefaults::default(),
    };
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return AgentDefaults::default(),
    };
    AgentDefaults {
        default_model_id: v["default_model_id"].as_str().unwrap_or("").to_owned(),
        default_provider_id: v["default_provider_id"].as_str().unwrap_or("").to_owned(),
        allow_project_override: v["allow_project_override"].as_bool().unwrap_or(true),
    }
}

fn save_defaults_async(d: AgentDefaults, ctx: &mut ViewContext<AgentDefaultsPageView>) {
    ctx.spawn(
        async move {
            let json = serde_json::json!({
                "default_model_id": d.default_model_id,
                "default_provider_id": d.default_provider_id,
                "allow_project_override": d.allow_project_override,
            });
            let text = serde_json::to_string_pretty(&json).unwrap_or_default();
            if let Some(path) = defaults_path() {
                if let Some(p) = path.parent() {
                    let _ = tokio::fs::create_dir_all(p).await;
                }
                let _ = tokio::fs::write(path, text).await;
            }
        },
        |_, _, _| {},
    );
}

// ── Actions ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AgentDefaultsAction {
    ToggleProjectOverride,
    ClearModel,
    ClearProvider,
}

// ── View ──────────────────────────────────────────────────────────────────────

pub struct AgentDefaultsPageView {
    page: PageType<Self>,
    defaults: AgentDefaults,
    model_input: ViewHandle<SubmittableTextInput>,
    provider_input: ViewHandle<SubmittableTextInput>,
    toggle_hover: MouseStateHandle,
    clear_model_hover: MouseStateHandle,
    clear_provider_hover: MouseStateHandle,
}

impl AgentDefaultsPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let loaded = load_defaults();

        let model_id = loaded.default_model_id.clone();
        let provider_id = loaded.default_provider_id.clone();

        let model_input = ctx.add_typed_action_view(move |ctx| {
            let mut i = SubmittableTextInput::new(ctx);
            i.set_placeholder_text(
                "e.g. gpt-4.1, claude-sonnet-4-5, gemini-2.5-pro".to_owned(),
                ctx,
            );
            if !model_id.is_empty() {
                i.editor()
                    .update(ctx, |ed, ctx| ed.set_buffer_text(&model_id, ctx));
            }
            i
        });

        let provider_input = ctx.add_typed_action_view(move |ctx| {
            let mut i = SubmittableTextInput::new(ctx);
            i.set_placeholder_text(
                "e.g. openai, anthropic, google (leave blank = auto)".to_owned(),
                ctx,
            );
            if !provider_id.is_empty() {
                i.editor()
                    .update(ctx, |ed, ctx| ed.set_buffer_text(&provider_id, ctx));
            }
            i
        });

        ctx.subscribe_to_view(
            &model_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    me.defaults.default_model_id = text.trim().to_owned();
                    save_defaults_async(me.defaults.clone(), ctx);
                    ctx.notify();
                }
            },
        );

        ctx.subscribe_to_view(
            &provider_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    me.defaults.default_provider_id = text.trim().to_owned();
                    save_defaults_async(me.defaults.clone(), ctx);
                    ctx.notify();
                }
            },
        );

        Self {
            page: PageType::new_monolith(AgentDefaultsWidget::default(), None, true),
            defaults: loaded,
            model_input,
            provider_input,
            toggle_hover: Default::default(),
            clear_model_hover: Default::default(),
            clear_provider_hover: Default::default(),
        }
    }
}

impl Entity for AgentDefaultsPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for AgentDefaultsPageView {
    type Action = AgentDefaultsAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            AgentDefaultsAction::ToggleProjectOverride => {
                self.defaults.allow_project_override = !self.defaults.allow_project_override;
                save_defaults_async(self.defaults.clone(), ctx);
                ctx.notify();
            }
            AgentDefaultsAction::ClearModel => {
                self.defaults.default_model_id = String::new();
                self.model_input.update(ctx, |i, ctx| {
                    i.editor().update(ctx, |ed, ctx| ed.clear_buffer(ctx))
                });
                save_defaults_async(self.defaults.clone(), ctx);
                ctx.notify();
            }
            AgentDefaultsAction::ClearProvider => {
                self.defaults.default_provider_id = String::new();
                self.provider_input.update(ctx, |i, ctx| {
                    i.editor().update(ctx, |ed, ctx| ed.clear_buffer(ctx))
                });
                save_defaults_async(self.defaults.clone(), ctx);
                ctx.notify();
            }
        }
    }
}

impl View for AgentDefaultsPageView {
    fn ui_name() -> &'static str {
        "AgentDefaultsPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

// ── Widget ────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct AgentDefaultsWidget {}

impl SettingsWidget for AgentDefaultsWidget {
    type View = AgentDefaultsPageView;

    fn search_terms(&self) -> &str {
        "agent defaults global model provider override project default session"
    }

    fn render(
        &self,
        view: &AgentDefaultsPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let font = appearance.ui_font_family();
        let mono = appearance.monospace_font_family();
        let dim = theme.disabled_ui_text_color();
        let active = theme.active_ui_text_color();
        let accent: Fill = theme.accent().into_solid().into();
        let d = &view.defaults;

        // ── Header ────────────────────────────────────────────────────────
        let hdr = build_sub_header(appearance, "Global Agent Defaults", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let desc = Container::new(
            Text::new(
                "Applied when opening a new agent session that has no previous project \
                 override.  Individual projects can customise these via scaffold.yml \
                 or .specsmith/agent_defaults.json when project overrides are enabled."
                    .to_string(),
                font,
                12.,
            )
            .with_color(dim.into())
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_bottom(14.)
        .finish();

        // ── Default model field ───────────────────────────────────────────
        let model_hdr = build_sub_header(appearance, "Default Model", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let current_model_hint = if d.default_model_id.is_empty() {
            "None set — auto-selected by provider".to_owned()
        } else {
            format!("Current: {}", d.default_model_id)
        };

        let clear_model_hover = view.clear_model_hover.clone();
        let clear_model_btn: Box<dyn Element> = Hoverable::new(clear_model_hover, move |ts| {
            Text::new_inline("Clear".to_string(), font, 11.)
                .with_color(if ts.is_hovered() {
                    active.into()
                } else {
                    dim.into()
                })
                .finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(|ctx, _, _| ctx.dispatch_typed_action(AgentDefaultsAction::ClearModel))
        .finish();

        let model_card = Container::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(ChildView::new(&view.model_input).finish())
                .with_child(
                    Container::new(
                        Flex::row()
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_child(
                                Expanded::new(
                                    1.,
                                    Text::new_inline(current_model_hint, mono, 10.)
                                        .with_color(dim.into())
                                        .finish(),
                                )
                                .finish(),
                            )
                            .with_child(clear_model_btn)
                            .finish(),
                    )
                    .with_margin_top(4.)
                    .finish(),
                )
                .finish(),
        )
        .with_background(theme.surface_1())
        .with_uniform_padding(12.)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
        .with_margin_bottom(12.)
        .finish();

        // ── Default provider field ────────────────────────────────────────
        let provider_hdr = build_sub_header(appearance, "Default Provider", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let current_provider_hint = if d.default_provider_id.is_empty() {
            "None set — resolved from model ID".to_owned()
        } else {
            format!("Current: {}", d.default_provider_id)
        };

        let clear_prov_hover = view.clear_provider_hover.clone();
        let clear_prov_btn: Box<dyn Element> = Hoverable::new(clear_prov_hover, move |ts| {
            Text::new_inline("Clear".to_string(), font, 11.)
                .with_color(if ts.is_hovered() {
                    active.into()
                } else {
                    dim.into()
                })
                .finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(|ctx, _, _| ctx.dispatch_typed_action(AgentDefaultsAction::ClearProvider))
        .finish();

        let provider_card = Container::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(ChildView::new(&view.provider_input).finish())
                .with_child(
                    Container::new(
                        Flex::row()
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_child(
                                Expanded::new(
                                    1.,
                                    Text::new_inline(current_provider_hint, mono, 10.)
                                        .with_color(dim.into())
                                        .finish(),
                                )
                                .finish(),
                            )
                            .with_child(clear_prov_btn)
                            .finish(),
                    )
                    .with_margin_top(4.)
                    .finish(),
                )
                .finish(),
        )
        .with_background(theme.surface_1())
        .with_uniform_padding(12.)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
        .with_margin_bottom(12.)
        .finish();

        // ── Per-project override toggle ───────────────────────────────────
        let override_hdr = build_sub_header(appearance, "Per-Project Override", None)
            .with_padding_bottom(HEADER_PADDING)
            .finish();

        let override_on = d.allow_project_override;
        let toggle_label = if override_on { "Enabled" } else { "Disabled" };
        let toggle_color: Fill = if override_on { accent } else { dim.into() };
        let toggle_hover = view.toggle_hover.clone();

        let toggle_row = Hoverable::new(toggle_hover, move |ts| {
            let bg = if ts.is_hovered() {
                theme.surface_2()
            } else {
                theme.surface_1()
            };
            Container::new(
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(
                        Expanded::new(
                            1.,
                            Flex::column()
                                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                                .with_child(
                                    Text::new_inline(
                                        "Allow projects to override global defaults".to_string(),
                                        font,
                                        12.,
                                    )
                                    .with_style(Properties::default().weight(Weight::Semibold))
                                    .with_color(active.into())
                                    .finish(),
                                )
                                .with_child(
                                    Container::new(
                                        Text::new(
                                            "When enabled, scaffold.yml or \
                                             .specsmith/agent_defaults.json in a project \
                                             directory will override these global settings."
                                                .to_string(),
                                            font,
                                            11.,
                                        )
                                        .with_color(dim.into())
                                        .soft_wrap(true)
                                        .finish(),
                                    )
                                    .with_margin_top(2.)
                                    .finish(),
                                )
                                .finish(),
                        )
                        .finish(),
                    )
                    .with_child(
                        Container::new(
                            Text::new_inline(toggle_label.to_string(), font, 12.)
                                .with_style(Properties::default().weight(Weight::Semibold))
                                .with_color(toggle_color.into())
                                .finish(),
                        )
                        .with_margin_left(16.)
                        .finish(),
                    )
                    .finish(),
            )
            .with_background(bg)
            .with_uniform_padding(12.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
            .finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(|ctx, _, _| ctx.dispatch_typed_action(AgentDefaultsAction::ToggleProjectOverride))
        .finish();

        // ── Info note ─────────────────────────────────────────────────────
        let note = Container::new(
            Text::new(
                "\u{2139}\u{FE0F}  Provider management (API keys, Ollama, custom endpoints) \
                 is in Specsmith \u{2192} AI Providers."
                    .to_string(),
                font,
                11.,
            )
            .with_color(dim.into())
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_top(16.)
        .finish();

        // ── Assemble ──────────────────────────────────────────────────────
        Container::new(
            ConstrainedBox::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(hdr)
                    .with_child(desc)
                    .with_child(model_hdr)
                    .with_child(model_card)
                    .with_child(render_separator(appearance))
                    .with_child(provider_hdr)
                    .with_child(provider_card)
                    .with_child(render_separator(appearance))
                    .with_child(override_hdr)
                    .with_child(toggle_row)
                    .with_child(note)
                    .finish(),
            )
            .with_max_width(720.)
            .finish(),
        )
        .with_uniform_padding(28.)
        .finish()
    }
}

// ── Settings metadata ─────────────────────────────────────────────────────────

impl SettingsPageMeta for AgentDefaultsPageView {
    fn section() -> SettingsSection {
        SettingsSection::AgentGlobalDefaults
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn on_page_selected(&mut self, _: bool, _ctx: &mut ViewContext<Self>) {}

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

impl From<ViewHandle<AgentDefaultsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AgentDefaultsPageView>) -> Self {
        SettingsPageViewHandle::AgentGlobalDefaults(view_handle)
    }
}
