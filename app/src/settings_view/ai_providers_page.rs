//! AI Providers settings page — card-based provider registry.
//!
//! Modelled after the glossa-lab ProvidersPanel UX:
//!  - Expandable cards per provider (type icon, name, status badge, enabled toggle, Test button)
//!  - Expanded card: editable name / base_url / api_key, available-models chips, Delete button
//!  - Top bar: Detect Ollama + Add Provider buttons
//!  - Inline add-provider form with type selector
//!
//! Persistence: ~/.specsmith/providers.json

use super::{
    settings_page::{
        MatchData, PageType, SettingsPageEvent, SettingsPageMeta, SettingsPageViewHandle,
        SettingsWidget, CONTENT_FONT_SIZE,
    },
    SettingsSection,
};
use crate::appearance::Appearance;
use crate::ui_components::blended_colors;
use crate::view_components::action_button::{ActionButton, NakedTheme};
use crate::view_components::{SubmittableTextInput, SubmittableTextInputEvent};
use warp_core::ui::theme::color::internal_colors;
use warpui::{
    elements::{
        Border, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element,
        Expanded, Flex, Hoverable, MouseStateHandle, ParentElement, Radius, Text,
    },
    fonts::{Properties, Weight},
    platform::Cursor,
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
    },
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

// ── Provider type helpers ──────────────────────────────────────────────────────

fn type_icon(pt: &str) -> &'static str {
    match pt {
        "cloud" => "\u{2601}",        // ☁
        "ollama" => "\u{1F999}",      // 🦙
        "byoe" => "\u{1F50C}",        // 🔌
        "huggingface" => "\u{1F917}", // 🤗
        _ => "\u{1F517}",             // 🔗
    }
}

fn type_label(pt: &str) -> &'static str {
    match pt {
        "cloud" => "cloud",
        "ollama" => "ollama",
        "byoe" => "byoe",
        "huggingface" => "hf",
        _ => "custom",
    }
}

fn status_label(s: &str) -> &'static str {
    match s {
        "reachable" => "\u{25CF} reachable",
        "unreachable" => "\u{25CF} unreachable",
        _ => "\u{25CB} untested",
    }
}

// ── Persistence ────────────────────────────────────────────────────────────────

fn providers_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".specsmith").join("providers.json"))
}

fn load_providers() -> Option<Vec<AiModelEntry>> {
    let path = providers_path()?;
    let text = std::fs::read_to_string(&path).ok()?;
    let arr: Vec<serde_json::Value> = serde_json::from_str(&text).ok()?;
    Some(
        arr.into_iter()
            .filter_map(|v| {
                let name = v["name"].as_str()?.to_owned();
                let id = v["id"]
                    .as_str()
                    .or_else(|| v["provider_id"].as_str())
                    .unwrap_or(&name)
                    .to_owned();
                Some(AiModelEntry {
                    name,
                    id,
                    provider_type: v["provider_type"].as_str().unwrap_or("cloud").to_owned(),
                    base_url: v["base_url"].as_str().unwrap_or("").to_owned(),
                    api_key_set: v["api_key_set"].as_bool().unwrap_or(false),
                    enabled: v["enabled"].as_bool().unwrap_or(true),
                    status: v["status"].as_str().unwrap_or("untested").to_owned(),
                    available_models: v["available_models"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|m| m.as_str().map(|s| s.to_owned()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    context_tokens: v["context_tokens"].as_u64(),
                    output_tokens: v["output_tokens"].as_u64(),
                    row_hover: MouseStateHandle::default(),
                    toggle_hover: MouseStateHandle::default(),
                })
            })
            .collect(),
    )
}

fn serialize_providers(models: &[AiModelEntry]) -> String {
    let arr: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            let mut obj = serde_json::json!({
                "name": m.name,
                "id": m.id,
                "provider_type": m.provider_type,
                "base_url": m.base_url,
                "api_key_set": m.api_key_set,
                "enabled": m.enabled,
                "status": m.status,
                "available_models": m.available_models,
            });
            if let Some(c) = m.context_tokens {
                obj["context_tokens"] = c.into();
            }
            if let Some(o) = m.output_tokens {
                obj["output_tokens"] = o.into();
            }
            obj
        })
        .collect();
    serde_json::to_string_pretty(&arr).unwrap_or_default()
}

// ── Data model ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct AiModelEntry {
    pub name: String,
    pub id: String,
    pub provider_type: String,
    pub base_url: String,
    pub api_key_set: bool,
    pub enabled: bool,
    pub status: String,
    pub available_models: Vec<String>,
    pub context_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub row_hover: MouseStateHandle,
    pub toggle_hover: MouseStateHandle,
}

impl AiModelEntry {
    fn new_cloud(name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
            provider_type: "cloud".to_owned(),
            base_url: String::new(),
            api_key_set: false,
            enabled: true,
            status: "untested".to_owned(),
            available_models: vec![],
            context_tokens: None,
            output_tokens: None,
            row_hover: Default::default(),
            toggle_hover: Default::default(),
        }
    }
}

// ── Bucket scores (REQ-281) ────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct BucketScore {
    pub reasoning: f32,
    pub conversational: f32,
    pub longform: f32,
}

fn load_bucket_scores() -> std::collections::HashMap<String, BucketScore> {
    let path = match dirs::home_dir() {
        Some(h) => h.join(".specsmith").join("model_scores.json"),
        None => return Default::default(),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Default::default(),
    };
    let root: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Default::default(),
    };
    let bucket_scores = match root.get("bucket_scores").and_then(|b| b.as_object()) {
        Some(m) => m,
        None => return Default::default(),
    };
    bucket_scores
        .iter()
        .filter_map(|(k, v)| {
            Some((
                k.clone(),
                BucketScore {
                    reasoning: v["reasoning_score"].as_f64().unwrap_or(0.0) as f32,
                    conversational: v["conversational_score"].as_f64().unwrap_or(0.0) as f32,
                    longform: v["longform_score"].as_f64().unwrap_or(0.0) as f32,
                },
            ))
        })
        .collect()
}

// ── Actions ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AiProvidersPageAction {
    ToggleExpand(usize),
    ToggleEnabled(usize),
    TestProvider(usize),
    DeleteProvider(usize),
    ShowAddForm,
    HideAddForm,
    SetAddType(String),
    AddProvider,
    DetectOllama,
    SyncModelIntel,
}

// ── View ───────────────────────────────────────────────────────────────────────

pub struct AiProvidersPageView {
    page: PageType<Self>,
    pub models: Vec<AiModelEntry>,
    pub expanded_index: Option<usize>,
    pub show_add_form: bool,
    pub add_type: String,
    pub testing_index: Option<usize>,
    // add-form
    add_name_input: ViewHandle<SubmittableTextInput>,
    add_url_input: ViewHandle<SubmittableTextInput>,
    add_key_input: ViewHandle<SubmittableTextInput>,
    // edit-form (shared, repopulated on expansion)
    edit_name_input: ViewHandle<SubmittableTextInput>,
    edit_url_input: ViewHandle<SubmittableTextInput>,
    edit_key_input: ViewHandle<SubmittableTextInput>,
    // top bar
    detect_button: ViewHandle<ActionButton>,
    add_provider_button: ViewHandle<ActionButton>,
    sync_intel_button: ViewHandle<ActionButton>,
    pub bucket_scores: std::collections::HashMap<String, BucketScore>,
}

impl AiProvidersPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let mk_input = |ctx: &mut ViewContext<Self>, ph: &'static str| {
            ctx.add_typed_action_view(move |ctx| {
                let mut i = SubmittableTextInput::new(ctx);
                i.set_placeholder_text(ph.to_owned(), ctx);
                i
            })
        };

        let add_name_input = mk_input(ctx, "Provider name (e.g. My vLLM)");
        let add_url_input = mk_input(ctx, "Base URL (e.g. http://localhost:11434)");
        let add_key_input = mk_input(ctx, "API key (optional)");
        let edit_name_input = mk_input(ctx, "Rename provider");
        let edit_url_input = mk_input(ctx, "Base URL");
        let edit_key_input = mk_input(ctx, "Paste new API key to replace");

        // Subscribe: notify on add-form typing so the UI stays live
        ctx.subscribe_to_view(
            &add_name_input,
            |_, _, _: &SubmittableTextInputEvent, ctx| {
                ctx.notify();
            },
        );

        // Subscribe: save edits to the expanded provider
        ctx.subscribe_to_view(
            &edit_name_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    if let Some(idx) = me.expanded_index {
                        if let Some(m) = me.models.get_mut(idx) {
                            m.name = text.clone();
                            me.save(ctx);
                        }
                    }
                    ctx.notify();
                }
            },
        );
        ctx.subscribe_to_view(
            &edit_url_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    if let Some(idx) = me.expanded_index {
                        if let Some(m) = me.models.get_mut(idx) {
                            m.base_url = text.clone();
                            me.save(ctx);
                        }
                    }
                    ctx.notify();
                }
            },
        );
        ctx.subscribe_to_view(
            &edit_key_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    if !text.is_empty() {
                        if let Some(idx) = me.expanded_index {
                            if let Some(m) = me.models.get_mut(idx) {
                                m.api_key_set = true;
                                let id = m.id.clone();
                                let key = text.clone();
                                ctx.spawn(
                                    async move {
                                        let _ = tokio::process::Command::new("specsmith")
                                            .args([
                                                "config",
                                                "set",
                                                &format!("provider.{id}.api_key"),
                                                &key,
                                            ])
                                            .output()
                                            .await;
                                    },
                                    |_, _, _| {},
                                );
                                me.save(ctx);
                            }
                        }
                        ctx.notify();
                    }
                }
            },
        );

        let detect_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("\u{1F999} Detect Ollama", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::DetectOllama))
        });
        let add_provider_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("+ Add Provider", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::ShowAddForm))
        });
        let sync_intel_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Sync Scores", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::SyncModelIntel))
        });

        let models = load_providers().unwrap_or_else(|| {
            vec![
                AiModelEntry::new_cloud("GPT-4.1", "gpt-4.1"),
                AiModelEntry::new_cloud("o3", "o3"),
                AiModelEntry::new_cloud("o4-mini", "o4-mini"),
                AiModelEntry::new_cloud("Claude 3.7 Sonnet", "claude-3-7-sonnet-20250219"),
                AiModelEntry::new_cloud("Gemini 2.5 Pro", "gemini-2.5-pro"),
                {
                    let mut m = AiModelEntry::new_cloud("Ollama (local)", "ollama");
                    m.provider_type = "ollama".to_owned();
                    m.base_url = "http://localhost:11434".to_owned();
                    m
                },
            ]
        });

        Self {
            page: PageType::new_monolith(AiProvidersPageWidget, None, true),
            models,
            expanded_index: None,
            show_add_form: false,
            add_type: "cloud".to_owned(),
            testing_index: None,
            add_name_input,
            add_url_input,
            add_key_input,
            edit_name_input,
            edit_url_input,
            edit_key_input,
            detect_button,
            add_provider_button,
            sync_intel_button,
            bucket_scores: load_bucket_scores(),
        }
    }

    fn save(&self, ctx: &mut ViewContext<Self>) {
        #[cfg(not(target_family = "wasm"))]
        {
            let json = serialize_providers(&self.models);
            ctx.spawn(
                async move {
                    if let Some(path) = providers_path() {
                        if let Some(p) = path.parent() {
                            let _ = tokio::fs::create_dir_all(p).await;
                        }
                        let _ = tokio::fs::write(&path, json).await;
                    }
                },
                |_, _, _| {},
            );
        }
    }

    fn populate_edit_editors(&self, idx: usize, ctx: &mut ViewContext<Self>) {
        if let Some(m) = self.models.get(idx) {
            let name = m.name.clone();
            let url = m.base_url.clone();
            self.edit_name_input.update(ctx, |input, ctx| {
                input
                    .editor()
                    .update(ctx, |ed, ctx| ed.set_buffer_text(&name, ctx));
            });
            self.edit_url_input.update(ctx, |input, ctx| {
                input
                    .editor()
                    .update(ctx, |ed, ctx| ed.set_buffer_text(&url, ctx));
            });
        }
    }
}

impl Entity for AiProvidersPageView {
    type Event = SettingsPageEvent;
}

impl View for AiProvidersPageView {
    fn ui_name() -> &'static str {
        "AiProvidersPage"
    }
    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl TypedActionView for AiProvidersPageView {
    type Action = AiProvidersPageAction;

    fn handle_action(&mut self, action: &AiProvidersPageAction, ctx: &mut ViewContext<Self>) {
        match action {
            AiProvidersPageAction::ToggleExpand(idx) => {
                let idx = *idx;
                self.expanded_index = if self.expanded_index == Some(idx) {
                    None
                } else {
                    self.populate_edit_editors(idx, ctx);
                    Some(idx)
                };
                ctx.notify();
            }
            AiProvidersPageAction::ToggleEnabled(idx) => {
                if let Some(m) = self.models.get_mut(*idx) {
                    m.enabled = !m.enabled;
                    self.save(ctx);
                }
                ctx.notify();
            }
            AiProvidersPageAction::TestProvider(idx) => {
                let idx = *idx;
                if let Some(m) = self.models.get(idx) {
                    let id = m.id.clone();
                    self.testing_index = Some(idx);
                    ctx.notify();
                    ctx.spawn(
                        async move {
                            tokio::process::Command::new("specsmith")
                                .args(["provider", "test", &id, "--json"])
                                .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                                .output()
                                .await
                                .map(|o| {
                                    let text = String::from_utf8_lossy(&o.stdout).to_string();
                                    o.status.success()
                                        || text.contains("reachable")
                                        || text.contains("\"valid\":true")
                                })
                        },
                        move |me, result, ctx| {
                            if me.testing_index == Some(idx) {
                                me.testing_index = None;
                            }
                            if let Some(m) = me.models.get_mut(idx) {
                                m.status = match result {
                                    Ok(true) => "reachable",
                                    _ => "unreachable",
                                }
                                .to_owned();
                                me.save(ctx);
                            }
                            ctx.notify();
                        },
                    );
                }
            }
            AiProvidersPageAction::DeleteProvider(idx) => {
                let idx = *idx;
                if idx < self.models.len() {
                    self.models.remove(idx);
                    self.expanded_index = match self.expanded_index {
                        Some(ei) if ei == idx => None,
                        Some(ei) if ei > idx => Some(ei - 1),
                        other => other,
                    };
                    self.save(ctx);
                }
                ctx.notify();
            }
            AiProvidersPageAction::ShowAddForm => {
                self.show_add_form = true;
                ctx.notify();
            }
            AiProvidersPageAction::HideAddForm => {
                self.show_add_form = false;
                ctx.notify();
            }
            AiProvidersPageAction::SetAddType(t) => {
                self.add_type = t.clone();
                let default_url = match t.as_str() {
                    "ollama" => "http://localhost:11434",
                    "byoe" => "http://localhost:8000/v1",
                    _ => "",
                }
                .to_owned();
                self.add_url_input.update(ctx, |input, ctx| {
                    input
                        .editor()
                        .update(ctx, |ed, ctx| ed.set_buffer_text(&default_url, ctx));
                });
                ctx.notify();
            }
            AiProvidersPageAction::AddProvider => {
                let name = self
                    .add_name_input
                    .as_ref(ctx)
                    .editor()
                    .as_ref(ctx)
                    .buffer_text(ctx);
                let url = self
                    .add_url_input
                    .as_ref(ctx)
                    .editor()
                    .as_ref(ctx)
                    .buffer_text(ctx);
                if !name.trim().is_empty() {
                    let mut entry = AiModelEntry::new_cloud(name.trim(), name.trim());
                    entry.provider_type = self.add_type.clone();
                    entry.base_url = url.trim().to_owned();
                    self.models.push(entry);
                    self.show_add_form = false;
                    self.add_name_input.update(ctx, |i, ctx| {
                        i.editor().update(ctx, |ed, ctx| ed.clear_buffer(ctx))
                    });
                    self.add_url_input.update(ctx, |i, ctx| {
                        i.editor().update(ctx, |ed, ctx| ed.clear_buffer(ctx))
                    });
                    self.save(ctx);
                }
                ctx.notify();
            }
            AiProvidersPageAction::DetectOllama => {
                #[cfg(not(target_family = "wasm"))]
                ctx.spawn(
                    async move {
                        // Try specsmith first, then direct HTTP probe
                        if let Ok(out) = tokio::process::Command::new("specsmith")
                            .args(["ollama", "available", "--json"])
                            .env("SPECSMITH_NO_AUTO_UPDATE", "1")
                            .output()
                            .await
                        {
                            if out.status.success() {
                                return Ok((
                                    "http://localhost:11434".to_owned(),
                                    String::from_utf8_lossy(&out.stdout).to_string(),
                                ));
                            }
                        }
                        for port in [11434u16, 11435, 11436] {
                            let url = format!("http://localhost:{port}");
                            if let Ok(out) = tokio::process::Command::new("curl")
                                .args(["-s", "--max-time", "2", &format!("{url}/api/tags")])
                                .output()
                                .await
                            {
                                if out.status.success() {
                                    return Ok((url, String::new()));
                                }
                            }
                        }
                        Err("No Ollama instance detected".to_owned())
                    },
                    |me, result, ctx| {
                        if let Ok((base_url, _)) = result {
                            if !me
                                .models
                                .iter()
                                .any(|m| m.provider_type == "ollama" && m.base_url == base_url)
                            {
                                let mut entry = AiModelEntry::new_cloud("Ollama", "ollama-local");
                                entry.provider_type = "ollama".to_owned();
                                entry.base_url = base_url;
                                entry.status = "reachable".to_owned();
                                me.models.push(entry);
                                me.save(ctx);
                            }
                        }
                        ctx.notify();
                    },
                );
            }
            AiProvidersPageAction::SyncModelIntel => {
                #[cfg(not(target_family = "wasm"))]
                ctx.spawn(
                    async move {
                        tokio::process::Command::new("specsmith")
                            .args(["model-intel", "sync", "--json"])
                            .output()
                            .await
                    },
                    |me, result, ctx| {
                        if let Ok(out) = result {
                            if out.status.success() {
                                me.bucket_scores = load_bucket_scores();
                                ctx.notify();
                            }
                        }
                    },
                );
            }
        }
    }
}

impl SettingsPageMeta for AiProvidersPageView {
    fn section() -> SettingsSection {
        SettingsSection::AiProviders
    }
    fn should_render(&self, _: &AppContext) -> bool {
        true
    }
    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }
    fn scroll_to_widget(&mut self, id: &'static str) {
        self.page.scroll_to_widget(id);
    }
    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<AiProvidersPageView>> for SettingsPageViewHandle {
    fn from(h: ViewHandle<AiProvidersPageView>) -> Self {
        SettingsPageViewHandle::AiProviders(h)
    }
}

// ── Widget ─────────────────────────────────────────────────────────────────────

struct AiProvidersPageWidget;

impl SettingsWidget for AiProvidersPageWidget {
    type View = AiProvidersPageView;

    fn search_terms(&self) -> &str {
        "providers models ai llm gpt claude gemini openai anthropic ollama byoe cloud endpoint test detect add delete enable disable api key base url"
    }

    fn render(
        &self,
        view: &AiProvidersPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let font = appearance.ui_font_family();
        let mono = appearance.monospace_font_family();
        let sub = blended_colors::text_sub(theme, theme.surface_1());
        let active = theme.active_ui_text_color();
        let accent = theme.accent().into_solid();
        let border = internal_colors::neutral_2(theme);
        let surface = theme.surface_1();
        let surface_color = theme.surface_1().into_solid();
        let hover_bg = internal_colors::neutral_1(theme);
        let err_col = theme.ui_error_color();

        // ── Page header ────────────────────────────────────────────────────
        let header_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Expanded::new(
                    1.,
                    Text::new(
                        "AI Provider Registry".to_string(),
                        font,
                        CONTENT_FONT_SIZE + 4.,
                    )
                    .with_style(Properties::default().weight(Weight::Semibold))
                    .with_color(active.into())
                    .finish(),
                )
                .finish(),
            )
            .with_child(
                Container::new(ChildView::new(&view.detect_button).finish())
                    .with_margin_right(6.)
                    .finish(),
            )
            .with_child(ChildView::new(&view.add_provider_button).finish())
            .finish();

        let page_desc = Container::new(
            Text::new(
                "Manage AI model providers. Saved to ~/.specsmith/providers.json.".to_string(),
                font,
                CONTENT_FONT_SIZE,
            )
            .with_color(sub)
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_bottom(12.)
        .finish();

        // ── Add-provider form ──────────────────────────────────────────────
        let add_form_elem: Option<Box<dyn Element>> = if view.show_add_form {
            let tabs = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(4.)
                .with_child(tab_btn(
                    "cloud",
                    &view.add_type,
                    font,
                    CONTENT_FONT_SIZE,
                    accent,
                    sub,
                ))
                .with_child(tab_btn(
                    "ollama",
                    &view.add_type,
                    font,
                    CONTENT_FONT_SIZE,
                    accent,
                    sub,
                ))
                .with_child(tab_btn(
                    "byoe",
                    &view.add_type,
                    font,
                    CONTENT_FONT_SIZE,
                    accent,
                    sub,
                ))
                .with_child(tab_btn(
                    "huggingface",
                    &view.add_type,
                    font,
                    CONTENT_FONT_SIZE,
                    accent,
                    sub,
                ))
                .finish();

            let add_btn = appearance
                .ui_builder()
                .button(ButtonVariant::Accent, MouseStateHandle::default())
                .with_style(UiComponentStyles {
                    font_size: Some(CONTENT_FONT_SIZE),
                    padding: Some(Coords::uniform(6.)),
                    ..Default::default()
                })
                .with_centered_text_label("Add".to_string())
                .build()
                .on_click(|ctx, _, _| ctx.dispatch_typed_action(AiProvidersPageAction::AddProvider))
                .finish();
            let cancel_btn = appearance
                .ui_builder()
                .button(ButtonVariant::Secondary, MouseStateHandle::default())
                .with_style(UiComponentStyles {
                    font_size: Some(CONTENT_FONT_SIZE),
                    padding: Some(Coords::uniform(6.)),
                    ..Default::default()
                })
                .with_centered_text_label("Cancel".to_string())
                .build()
                .on_click(|ctx, _, _| ctx.dispatch_typed_action(AiProvidersPageAction::HideAddForm))
                .finish();

            let form = Container::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(6.)
                    .with_child(
                        Text::new("Add Provider".to_string(), font, CONTENT_FONT_SIZE + 1.)
                            .with_style(Properties::default().weight(Weight::Semibold))
                            .with_color(active.into())
                            .finish(),
                    )
                    .with_child(tabs)
                    .with_child(labeled_input(
                        "Name",
                        &view.add_name_input,
                        font,
                        sub,
                        CONTENT_FONT_SIZE,
                    ))
                    .with_child(labeled_input(
                        "Base URL",
                        &view.add_url_input,
                        font,
                        sub,
                        CONTENT_FONT_SIZE,
                    ))
                    .with_child(labeled_input(
                        "API Key (opt.)",
                        &view.add_key_input,
                        font,
                        sub,
                        CONTENT_FONT_SIZE,
                    ))
                    .with_child(
                        Flex::row()
                            .with_spacing(6.)
                            .with_child(add_btn)
                            .with_child(cancel_btn)
                            .finish(),
                    )
                    .finish(),
            )
            .with_background(surface)
            .with_border(Border::all(1.).with_border_color(border))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_uniform_padding(14.)
            .with_margin_bottom(12.)
            .finish();

            Some(form)
        } else {
            None
        };

        // ── Provider cards ─────────────────────────────────────────────────
        let mut cards = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(6.);

        for (idx, model) in view.models.iter().enumerate() {
            let is_exp = view.expanded_index == Some(idx);
            let is_test = view.testing_index == Some(idx);

            let icon_str = type_icon(&model.provider_type).to_owned();
            let type_lbl = type_label(&model.provider_type).to_owned();
            let stat_lbl = if is_test {
                "Testing\u{2026}".to_owned()
            } else {
                status_label(&model.status).to_owned()
            };
            let stat_col = match model.status.as_str() {
                "reachable" => accent,
                "unreachable" => err_col,
                _ => sub,
            };

            let name_str = model.name.clone();
            let url_preview: String = if model.base_url.len() > 50 {
                format!("{}…", &model.base_url[..47])
            } else {
                model.base_url.clone()
            };
            let enabled = model.enabled;

            // -- collapsed header ---
            let header_elem = Hoverable::new(model.row_hover.clone(), move |state| {
                let bg = if state.is_hovered() {
                    hover_bg
                } else {
                    surface_color
                };
                Container::new(
                    Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        // type icon
                        .with_child(
                            Container::new(Text::new_inline(icon_str.clone(), font, 16.).finish())
                                .with_margin_right(8.)
                                .finish(),
                        )
                        // name + meta
                        .with_child(
                            Expanded::new(
                                1.,
                                Flex::column()
                                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                                    .with_child(
                                        Flex::row()
                                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                            .with_spacing(5.)
                                            .with_child(
                                                Text::new_inline(
                                                    name_str.clone(),
                                                    font,
                                                    CONTENT_FONT_SIZE + 1.,
                                                )
                                                .with_style(
                                                    Properties::default().weight(Weight::Semibold),
                                                )
                                                .with_color(active.into())
                                                .finish(),
                                            )
                                            .with_child(badge(
                                                &type_lbl,
                                                accent,
                                                font,
                                                CONTENT_FONT_SIZE - 1.,
                                            ))
                                            .finish(),
                                    )
                                    .with_child(
                                        Text::new_inline(
                                            stat_lbl.clone(),
                                            font,
                                            CONTENT_FONT_SIZE - 1.,
                                        )
                                        .with_color(stat_col)
                                        .finish(),
                                    )
                                    .finish(),
                            )
                            .finish(),
                        )
                        // enabled toggle
                        .with_child(
                            Container::new(
                                Hoverable::new(MouseStateHandle::default(), move |ts| {
                                    let lbl = if enabled { "[on]" } else { "[off]" };
                                    let col = if ts.is_hovered() {
                                        active.into()
                                    } else if enabled {
                                        accent
                                    } else {
                                        sub
                                    };
                                    Text::new_inline(lbl.to_string(), font, CONTENT_FONT_SIZE - 1.)
                                        .with_color(col)
                                        .finish()
                                })
                                .with_cursor(Cursor::PointingHand)
                                .on_click(move |ctx, _, _| {
                                    ctx.dispatch_typed_action(AiProvidersPageAction::ToggleEnabled(
                                        idx,
                                    ))
                                })
                                .finish(),
                            )
                            .with_margin_left(6.)
                            .with_margin_right(6.)
                            .finish(),
                        )
                        // test button
                        .with_child(
                            Container::new(
                                Hoverable::new(MouseStateHandle::default(), move |ts| {
                                    let lbl = if is_test { "\u{2026}" } else { "Test" };
                                    let col = if ts.is_hovered() { active.into() } else { sub };
                                    Text::new_inline(lbl.to_string(), font, CONTENT_FONT_SIZE - 1.)
                                        .with_color(col)
                                        .finish()
                                })
                                .with_cursor(Cursor::PointingHand)
                                .on_click(move |ctx, _, _| {
                                    ctx.dispatch_typed_action(AiProvidersPageAction::TestProvider(
                                        idx,
                                    ))
                                })
                                .finish(),
                            )
                            .with_margin_right(6.)
                            .finish(),
                        )
                        // expand chevron
                        .with_child(
                            Text::new_inline(
                                if is_exp { "\u{25B2}" } else { "\u{25BC}" },
                                font,
                                9.,
                            )
                            .with_color(sub)
                            .finish(),
                        )
                        .finish(),
                )
                .with_background_color(bg)
                .with_padding_top(8.)
                .with_padding_bottom(8.)
                .with_padding_left(10.)
                .with_padding_right(10.)
                .finish()
            })
            .with_cursor(Cursor::PointingHand)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(AiProvidersPageAction::ToggleExpand(idx))
            })
            .finish();

            // -- expanded details ---
            let detail_elem: Option<Box<dyn Element>> = if is_exp {
                let del_btn = appearance
                    .ui_builder()
                    .button(ButtonVariant::Secondary, MouseStateHandle::default())
                    .with_style(UiComponentStyles {
                        font_size: Some(CONTENT_FONT_SIZE - 1.),
                        padding: Some(Coords::uniform(5.)),
                        ..Default::default()
                    })
                    .with_centered_text_label("Delete Provider".to_string())
                    .build()
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(AiProvidersPageAction::DeleteProvider(idx))
                    })
                    .finish();

                let mut detail = Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(4.);

                if !url_preview.is_empty() {
                    detail.add_child(
                        Text::new_inline(
                            format!("URL: {url_preview}"),
                            mono,
                            CONTENT_FONT_SIZE - 1.,
                        )
                        .with_color(sub)
                        .finish(),
                    );
                }
                detail.add_child(labeled_input(
                    "Rename",
                    &view.edit_name_input,
                    font,
                    sub,
                    CONTENT_FONT_SIZE,
                ));
                detail.add_child(labeled_input(
                    "Base URL",
                    &view.edit_url_input,
                    font,
                    sub,
                    CONTENT_FONT_SIZE,
                ));
                detail.add_child(labeled_input(
                    "API Key",
                    &view.edit_key_input,
                    font,
                    sub,
                    CONTENT_FONT_SIZE,
                ));

                // available models chips
                if !model.available_models.is_empty() {
                    let mut chips = Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_spacing(4.);
                    for m in model.available_models.iter().take(12) {
                        chips.add_child(
                            Container::new(
                                Text::new_inline(m.clone(), mono, CONTENT_FONT_SIZE - 2.)
                                    .with_color(active.into())
                                    .finish(),
                            )
                            .with_background_color(internal_colors::neutral_2(theme))
                            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(3.)))
                            .with_horizontal_padding(5.)
                            .with_vertical_padding(2.)
                            .finish(),
                        );
                    }
                    if model.available_models.len() > 12 {
                        chips.add_child(
                            Text::new_inline(
                                format!("+{} more", model.available_models.len() - 12),
                                font,
                                CONTENT_FONT_SIZE - 2.,
                            )
                            .with_color(sub)
                            .finish(),
                        );
                    }
                    detail.add_child(Container::new(chips.finish()).with_margin_top(4.).finish());
                }

                detail.add_child(Container::new(del_btn).with_margin_top(6.).finish());

                Some(
                    Container::new(detail.finish())
                        .with_background(surface)
                        .with_border(Border::top(1.).with_border_color(border))
                        .with_uniform_padding(10.)
                        .finish(),
                )
            } else {
                None
            };

            // -- assemble card ---
            let mut card_col =
                Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
            card_col.add_child(header_elem);
            if let Some(d) = detail_elem {
                card_col.add_child(d);
            }

            cards.add_child(
                Container::new(card_col.finish())
                    .with_background(surface)
                    .with_border(Border::all(1.).with_border_color(border))
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
                    .finish(),
            );
        }

        if view.models.is_empty() && !view.show_add_form {
            cards.add_child(
                Container::new(Text::new(
                    "No providers configured. Click \u{201c}+ Add Provider\u{201d} to get started.".to_string(),
                    font, CONTENT_FONT_SIZE,
                ).with_color(sub).finish())
                .with_uniform_padding(20.)
                .with_border(Border::all(1.).with_border_color(border))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
                .finish()
            );
        }

        // ── Action bar (sync scores) ────────────────────────────────────────
        let action_bar = Container::new(
            Flex::row()
                .with_spacing(6.)
                .with_child(ChildView::new(&view.sync_intel_button).finish())
                .finish(),
        )
        .with_margin_top(8.)
        .finish();

        // ── Assemble ───────────────────────────────────────────────────────
        let mut page = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(Container::new(header_row).with_margin_bottom(4.).finish())
            .with_child(page_desc);

        if let Some(form) = add_form_elem {
            page.add_child(form);
        }
        page.add_child(cards.finish());
        page.add_child(action_bar);

        Container::new(
            ConstrainedBox::new(page.finish())
                .with_max_width(720.)
                .finish(),
        )
        .with_uniform_padding(28.)
        .finish()
    }
}

// ── Free helpers ────────────────────────────────────────────────────────────────

/// Type-selector tab button for the add form.
/// `unsel_bg` is the background color used for unselected state (typically `sub`).
fn tab_btn(
    t: &str,
    selected: &str,
    font: warpui::fonts::FamilyId,
    font_size: f32,
    accent: pathfinder_color::ColorU,
    unsel_bg: pathfinder_color::ColorU,
) -> Box<dyn Element> {
    let is_sel = t == selected;
    let icon = type_icon(t).to_owned();
    let lbl = type_label(t).to_owned();
    let label_s = format!("{icon} {lbl}");
    let t_owned = t.to_owned();

    Hoverable::new(MouseStateHandle::default(), move |state| {
        let bg = if is_sel || state.is_hovered() {
            accent
        } else {
            unsel_bg
        };
        let fg = if is_sel || state.is_hovered() {
            pathfinder_color::ColorU::white()
        } else {
            unsel_bg
        };
        Container::new(
            Text::new_inline(label_s.clone(), font, font_size - 1.)
                .with_color(if is_sel {
                    pathfinder_color::ColorU::white()
                } else {
                    fg
                })
                .finish(),
        )
        .with_background_color(bg)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
        .with_horizontal_padding(8.)
        .with_vertical_padding(4.)
        .finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(move |ctx, _, _| {
        ctx.dispatch_typed_action(AiProvidersPageAction::SetAddType(t_owned.clone()))
    })
    .finish()
}

/// Labeled text-input row.
fn labeled_input(
    label: &str,
    input: &ViewHandle<SubmittableTextInput>,
    font: warpui::fonts::FamilyId,
    sub: pathfinder_color::ColorU,
    font_size: f32,
) -> Box<dyn Element> {
    Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_child(
            Text::new_inline(label.to_string(), font, font_size - 1.)
                .with_style(Properties::default().weight(Weight::Semibold))
                .with_color(sub)
                .finish(),
        )
        .with_child(ChildView::new(input).finish())
        .finish()
}

/// Small coloured badge/pill.
fn badge(
    text: &str,
    bg: pathfinder_color::ColorU,
    font: warpui::fonts::FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    Container::new(
        Text::new_inline(text.to_string(), font, font_size)
            .with_color(pathfinder_color::ColorU::white())
            .finish(),
    )
    .with_background_color(bg)
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(3.)))
    .with_horizontal_padding(5.)
    .with_vertical_padding(2.)
    .finish()
}
