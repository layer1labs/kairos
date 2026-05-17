//! AI Providers settings page — 3-section provider registry.
//!
//! Sections:
//!  1. **Cloud Providers** — curated catalog (OpenAI, Anthropic, Google, Mistral, Cohere, xAI)
//!     with per-provider API key, status badge, Test / Scan-All buttons.
//!  2. **Ollama** — live detection, model list with VRAM filter, pull / delete.
//!  3. **Custom Endpoints (BYOE)** — presets (vLLM, LM Studio, Kobold…), add/edit/test,
//!     Discover button that probes common local ports.
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

    fn new_byoe(name: impl Into<String>, id: impl Into<String>, url: impl Into<String>) -> Self {
        let mut e = Self::new_cloud(name, id);
        e.provider_type = "byoe".to_owned();
        e.base_url = url.into();
        e
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

// ── Ollama model structures ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct OllamaModel {
    pub name: String,
    pub size_bytes: u64,
    pub param_size: String,
}

impl OllamaModel {
    fn size_display(&self) -> String {
        let gb = self.size_bytes as f64 / 1_073_741_824.0;
        if gb >= 1.0 {
            format!("{gb:.1} GB")
        } else {
            let mb = self.size_bytes as f64 / 1_048_576.0;
            format!("{mb:.0} MB")
        }
    }

    /// Conservative VRAM estimate: model weights + ~20% overhead.
    fn vram_gb_estimate(&self) -> f32 {
        (self.size_bytes as f64 / 1_073_741_824.0 * 1.2) as f32
    }
}

#[derive(Clone, Debug, Default)]
pub enum OllamaSectionStatus {
    #[default]
    Idle,
    Detecting,
    Online {
        url: String,
        models: Vec<OllamaModel>,
    },
    Offline,
    Pulling {
        model_name: String,
        progress: String,
    },
    PullDone {
        model_name: String,
    },
    PullError {
        model_name: String,
        error: String,
    },
    Deleting {
        model_name: String,
    },
}

impl OllamaSectionStatus {
    fn ollama_url(&self) -> Option<&str> {
        match self {
            Self::Online { url, .. } => Some(url.as_str()),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn models(&self) -> &[OllamaModel] {
        match self {
            Self::Online { models, .. } => models,
            _ => &[],
        }
    }
}

// ── BYOE preset URLs ───────────────────────────────────────────────────────────

const BYOE_PRESETS: &[(&str, &str)] = &[
    ("vLLM", "http://localhost:8000/v1"),
    ("LM Studio", "http://localhost:1234/v1"),
    ("Kobold", "http://localhost:5001/api"),
    ("LocalAI", "http://localhost:8080/v1"),
    ("Text Gen WebUI", "http://localhost:5000/v1"),
    ("Custom\u{2026}", ""),
];

const DISCOVER_PORTS: &[u16] = &[1234, 5000, 5001, 7860, 8000, 8080, 8088, 8888, 9000];

// ── Actions ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AiProvidersPageAction {
    // Cloud
    CloudToggleExpand(usize),
    CloudToggleEnabled(usize),
    CloudTestProvider(usize),
    ScanAllCloud,
    // Ollama
    DetectOllama,
    PullOllamaModel,
    DeleteOllamaModel(String),
    SetVramFilter(u32),
    // BYOE
    DiscoverEndpoints,
    ShowAddForm,
    HideAddForm,
    SetAddPreset(String),
    AddEndpoint,
    ByoeToggleExpand(usize),
    ByoeToggleEnabled(usize),
    ByoeTestProvider(usize),
    ByoeDeleteProvider(usize),
    // Misc
    SyncModelIntel,
}

// ── View ───────────────────────────────────────────────────────────────────────

pub struct AiProvidersPageView {
    page: PageType<Self>,
    pub cloud_providers: Vec<AiModelEntry>,
    pub cloud_expanded: Option<usize>,
    pub cloud_scanning: bool,
    pub cloud_testing: Option<usize>,
    pub endpoints: Vec<AiModelEntry>,
    pub endpoint_expanded: Option<usize>,
    pub endpoint_testing: Option<usize>,
    pub show_add_form: bool,
    pub add_preset_url: String,
    pub ollama: OllamaSectionStatus,
    pub vram_filter_gb: u32,
    cloud_edit_key_input: ViewHandle<SubmittableTextInput>,
    cloud_edit_url_input: ViewHandle<SubmittableTextInput>,
    byoe_edit_name_input: ViewHandle<SubmittableTextInput>,
    byoe_edit_url_input: ViewHandle<SubmittableTextInput>,
    byoe_edit_key_input: ViewHandle<SubmittableTextInput>,
    add_name_input: ViewHandle<SubmittableTextInput>,
    add_url_input: ViewHandle<SubmittableTextInput>,
    add_key_input: ViewHandle<SubmittableTextInput>,
    pull_model_input: ViewHandle<SubmittableTextInput>,
    scan_all_button: ViewHandle<ActionButton>,
    detect_ollama_button: ViewHandle<ActionButton>,
    discover_button: ViewHandle<ActionButton>,
    add_endpoint_button: ViewHandle<ActionButton>,
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

        let cloud_edit_key_input = mk_input(ctx, "Paste API key to save");
        let cloud_edit_url_input = mk_input(ctx, "Custom base URL (optional)");
        let byoe_edit_name_input = mk_input(ctx, "Endpoint name");
        let byoe_edit_url_input = mk_input(ctx, "Base URL");
        let byoe_edit_key_input = mk_input(ctx, "API key (optional)");
        let add_name_input = mk_input(ctx, "Endpoint name");
        let add_url_input = mk_input(ctx, "Base URL");
        let add_key_input = mk_input(ctx, "API key (optional)");
        let pull_model_input = mk_input(ctx, "Model name, e.g. llama3:8b");

        ctx.subscribe_to_view(
            &cloud_edit_key_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    if !text.is_empty() {
                        if let Some(idx) = me.cloud_expanded {
                            if let Some(m) = me.cloud_providers.get_mut(idx) {
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
        ctx.subscribe_to_view(
            &cloud_edit_url_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    if let Some(idx) = me.cloud_expanded {
                        if let Some(m) = me.cloud_providers.get_mut(idx) {
                            m.base_url = text.clone();
                            me.save(ctx);
                        }
                    }
                    ctx.notify();
                }
            },
        );
        ctx.subscribe_to_view(
            &byoe_edit_name_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    if let Some(idx) = me.endpoint_expanded {
                        if let Some(m) = me.endpoints.get_mut(idx) {
                            m.name = text.clone();
                            me.save(ctx);
                        }
                    }
                    ctx.notify();
                }
            },
        );
        ctx.subscribe_to_view(
            &byoe_edit_url_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    if let Some(idx) = me.endpoint_expanded {
                        if let Some(m) = me.endpoints.get_mut(idx) {
                            m.base_url = text.clone();
                            me.save(ctx);
                        }
                    }
                    ctx.notify();
                }
            },
        );
        ctx.subscribe_to_view(
            &byoe_edit_key_input,
            |me, _, ev: &SubmittableTextInputEvent, ctx| {
                if let SubmittableTextInputEvent::Submit(text) = ev {
                    if !text.is_empty() {
                        if let Some(idx) = me.endpoint_expanded {
                            if let Some(m) = me.endpoints.get_mut(idx) {
                                m.api_key_set = true;
                                m.status = "untested".to_owned();
                                me.save(ctx);
                            }
                        }
                        ctx.notify();
                    }
                }
            },
        );

        let scan_all_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("\u{2601} Scan All", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::ScanAllCloud))
        });
        let detect_ollama_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("\u{1F999} Detect Ollama", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::DetectOllama))
        });
        let discover_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("\u{1F50E} Discover", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::DiscoverEndpoints))
        });
        let add_endpoint_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("+ Add Endpoint", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::ShowAddForm))
        });
        let sync_intel_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Sync Scores", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::SyncModelIntel))
        });

        let all = load_providers().unwrap_or_else(default_providers);
        let mut cloud_providers: Vec<AiModelEntry> = all
            .iter()
            .filter(|m| m.provider_type == "cloud")
            .cloned()
            .collect();
        cloud_providers = ensure_cloud_catalog(cloud_providers);
        let endpoints: Vec<AiModelEntry> = all
            .into_iter()
            .filter(|m| m.provider_type != "cloud")
            .collect();

        Self {
            page: PageType::new_monolith(AiProvidersPageWidget, None, true),
            cloud_providers,
            cloud_expanded: None,
            cloud_scanning: false,
            cloud_testing: None,
            endpoints,
            endpoint_expanded: None,
            endpoint_testing: None,
            show_add_form: false,
            add_preset_url: String::new(),
            ollama: OllamaSectionStatus::default(),
            vram_filter_gb: 0,
            cloud_edit_key_input,
            cloud_edit_url_input,
            byoe_edit_name_input,
            byoe_edit_url_input,
            byoe_edit_key_input,
            add_name_input,
            add_url_input,
            add_key_input,
            pull_model_input,
            scan_all_button,
            detect_ollama_button,
            discover_button,
            add_endpoint_button,
            sync_intel_button,
            bucket_scores: load_bucket_scores(),
        }
    }

    fn save(&self, ctx: &mut ViewContext<Self>) {
        #[cfg(not(target_family = "wasm"))]
        {
            let mut all: Vec<AiModelEntry> = self.cloud_providers.clone();
            all.extend(self.endpoints.clone());
            let json = serialize_providers(&all);
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

    /// Spawns the Ollama detection. May be called from action handler and spawn callbacks.
    fn run_detect_ollama(&mut self, ctx: &mut ViewContext<Self>) {
        self.ollama = OllamaSectionStatus::Detecting;
        ctx.notify();
        ctx.spawn(
            async move {
                for port in [11434u16, 11435, 11436] {
                    let tags_url = format!("http://localhost:{port}/api/tags");
                    if let Ok(out) = tokio::process::Command::new("curl")
                        .args(["-s", "--max-time", "3", &tags_url])
                        .output()
                        .await
                    {
                        if out.status.success() && !out.stdout.is_empty() {
                            let text = String::from_utf8_lossy(&out.stdout).to_string();
                            let base_url = format!("http://localhost:{port}");
                            return Ok((base_url, text));
                        }
                    }
                }
                Err("No Ollama instance found on ports 11434-11436".to_owned())
            },
            |me, result, ctx| {
                me.ollama = match result {
                    Err(_) => OllamaSectionStatus::Offline,
                    Ok((url, json)) => {
                        let models = parse_ollama_models(&json);
                        OllamaSectionStatus::Online { url, models }
                    }
                };
                ctx.notify();
            },
        );
    }

    fn populate_cloud_edit(&self, idx: usize, ctx: &mut ViewContext<Self>) {
        if let Some(m) = self.cloud_providers.get(idx) {
            let url = m.base_url.clone();
            self.cloud_edit_url_input.update(ctx, |input, ctx| {
                input
                    .editor()
                    .update(ctx, |ed, ctx| ed.set_buffer_text(&url, ctx));
            });
        }
    }

    fn populate_byoe_edit(&self, idx: usize, ctx: &mut ViewContext<Self>) {
        if let Some(m) = self.endpoints.get(idx) {
            let name = m.name.clone();
            let url = m.base_url.clone();
            self.byoe_edit_name_input.update(ctx, |input, ctx| {
                input
                    .editor()
                    .update(ctx, |ed, ctx| ed.set_buffer_text(&name, ctx));
            });
            self.byoe_edit_url_input.update(ctx, |input, ctx| {
                input
                    .editor()
                    .update(ctx, |ed, ctx| ed.set_buffer_text(&url, ctx));
            });
        }
    }

    fn spawn_test_provider(&self, idx: usize, is_cloud: bool, ctx: &mut ViewContext<Self>) {
        let id = if is_cloud {
            self.cloud_providers.get(idx).map(|m| m.id.clone())
        } else {
            self.endpoints.get(idx).map(|m| m.id.clone())
        };
        let id = match id {
            Some(i) => i,
            None => return,
        };
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
                let status = match result {
                    Ok(true) => "reachable",
                    _ => "unreachable",
                }
                .to_owned();
                if is_cloud {
                    if me.cloud_testing == Some(idx) {
                        me.cloud_testing = None;
                    }
                    if let Some(m) = me.cloud_providers.get_mut(idx) {
                        m.status = status;
                        me.save(ctx);
                    }
                } else {
                    if me.endpoint_testing == Some(idx) {
                        me.endpoint_testing = None;
                    }
                    if let Some(m) = me.endpoints.get_mut(idx) {
                        m.status = status;
                        me.save(ctx);
                    }
                }
                ctx.notify();
            },
        );
    }
}

// ── Default cloud catalog ──────────────────────────────────────────────────────

fn default_providers() -> Vec<AiModelEntry> {
    vec![
        AiModelEntry::new_cloud("GPT-4.1", "gpt-4.1"),
        AiModelEntry::new_cloud("o3", "o3"),
        AiModelEntry::new_cloud("o4-mini", "o4-mini"),
        AiModelEntry::new_cloud("Claude 3.7 Sonnet", "claude-3-7-sonnet-20250219"),
        AiModelEntry::new_cloud("Gemini 2.5 Pro", "gemini-2.5-pro"),
        AiModelEntry::new_cloud("Mistral Large", "mistral-large-latest"),
        AiModelEntry::new_cloud("Cohere Command R+", "command-r-plus"),
        AiModelEntry::new_cloud("Grok 3", "grok-3"),
    ]
}

fn ensure_cloud_catalog(mut existing: Vec<AiModelEntry>) -> Vec<AiModelEntry> {
    for cat in default_providers() {
        if !existing.iter().any(|e| e.id == cat.id) {
            existing.push(cat);
        }
    }
    existing
}

fn parse_ollama_models(json: &str) -> Vec<OllamaModel> {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    v["models"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?.to_owned();
            let size_bytes = m["size"].as_u64().unwrap_or(0);
            let param_size = m["details"]["parameter_size"]
                .as_str()
                .unwrap_or("")
                .to_owned();
            Some(OllamaModel {
                name,
                size_bytes,
                param_size,
            })
        })
        .collect()
}

// ── Entity / View ─────────────────────────────────────────────────────────────

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

// ── Action handler ────────────────────────────────────────────────────────────

impl TypedActionView for AiProvidersPageView {
    type Action = AiProvidersPageAction;

    fn handle_action(&mut self, action: &AiProvidersPageAction, ctx: &mut ViewContext<Self>) {
        match action {
            // ── Cloud ──────────────────────────────────────────────────────────
            AiProvidersPageAction::CloudToggleExpand(idx) => {
                let idx = *idx;
                self.cloud_expanded = if self.cloud_expanded == Some(idx) {
                    None
                } else {
                    self.populate_cloud_edit(idx, ctx);
                    Some(idx)
                };
                ctx.notify();
            }
            AiProvidersPageAction::CloudToggleEnabled(idx) => {
                if let Some(m) = self.cloud_providers.get_mut(*idx) {
                    m.enabled = !m.enabled;
                    self.save(ctx);
                }
                ctx.notify();
            }
            AiProvidersPageAction::CloudTestProvider(idx) => {
                let idx = *idx;
                self.cloud_testing = Some(idx);
                ctx.notify();
                self.spawn_test_provider(idx, true, ctx);
            }
            AiProvidersPageAction::ScanAllCloud => {
                self.cloud_scanning = true;
                ctx.notify();
                let count = self.cloud_providers.len();
                for idx in 0..count {
                    self.spawn_test_provider(idx, true, ctx);
                }
                ctx.spawn(
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                    },
                    |me, _, ctx| {
                        me.cloud_scanning = false;
                        ctx.notify();
                    },
                );
            }

            // ── Ollama ─────────────────────────────────────────────────────────
            AiProvidersPageAction::DetectOllama => {
                self.run_detect_ollama(ctx);
            }
            AiProvidersPageAction::PullOllamaModel => {
                let model_name = self
                    .pull_model_input
                    .as_ref(ctx)
                    .editor()
                    .as_ref(ctx)
                    .buffer_text(ctx)
                    .trim()
                    .to_owned();
                if model_name.is_empty() {
                    return;
                }
                let ollama_url = self
                    .ollama
                    .ollama_url()
                    .unwrap_or("http://localhost:11434")
                    .to_owned();
                self.ollama = OllamaSectionStatus::Pulling {
                    model_name: model_name.clone(),
                    progress: "starting\u{2026}".to_owned(),
                };
                self.pull_model_input.update(ctx, |i, ctx| {
                    i.editor().update(ctx, |ed, ctx| ed.clear_buffer(ctx))
                });
                ctx.notify();
                ctx.spawn(
                    async move {
                        let pull_url = format!("{ollama_url}/api/pull");
                        let body = serde_json::json!({ "name": model_name }).to_string();
                        match tokio::process::Command::new("curl")
                            .args([
                                "-s",
                                "--max-time",
                                "600",
                                "-X",
                                "POST",
                                &pull_url,
                                "-H",
                                "Content-Type: application/json",
                                "-d",
                                &body,
                            ])
                            .output()
                            .await
                        {
                            Ok(out) if out.status.success() => Ok(model_name),
                            Ok(out) => {
                                Err((model_name, String::from_utf8_lossy(&out.stderr).to_string()))
                            }
                            Err(e) => Err((model_name, e.to_string())),
                        }
                    },
                    |me, result, ctx| {
                        match result {
                            Ok(name) => {
                                me.ollama = OllamaSectionStatus::PullDone { model_name: name };
                                me.run_detect_ollama(ctx);
                            }
                            Err((name, error)) => {
                                me.ollama = OllamaSectionStatus::PullError {
                                    model_name: name,
                                    error: error.chars().take(100).collect(),
                                };
                            }
                        }
                        ctx.notify();
                    },
                );
            }
            AiProvidersPageAction::DeleteOllamaModel(model_name) => {
                let model_name = model_name.clone();
                let ollama_url = self
                    .ollama
                    .ollama_url()
                    .unwrap_or("http://localhost:11434")
                    .to_owned();
                self.ollama = OllamaSectionStatus::Deleting {
                    model_name: model_name.clone(),
                };
                ctx.notify();
                ctx.spawn(
                    async move {
                        let delete_url = format!("{ollama_url}/api/delete");
                        let body = serde_json::json!({ "name": model_name }).to_string();
                        let _ = tokio::process::Command::new("curl")
                            .args([
                                "-s",
                                "--max-time",
                                "30",
                                "-X",
                                "DELETE",
                                &delete_url,
                                "-H",
                                "Content-Type: application/json",
                                "-d",
                                &body,
                            ])
                            .output()
                            .await;
                    },
                    |me, _, ctx| {
                        me.run_detect_ollama(ctx);
                    },
                );
            }
            AiProvidersPageAction::SetVramFilter(gb) => {
                self.vram_filter_gb = *gb;
                ctx.notify();
            }

            // ── BYOE ───────────────────────────────────────────────────────────
            AiProvidersPageAction::DiscoverEndpoints => {
                ctx.spawn(
                    async move {
                        let mut found: Vec<(String, String)> = vec![];
                        for &port in DISCOVER_PORTS {
                            let probe = format!("http://localhost:{port}/v1/models");
                            if let Ok(out) = tokio::process::Command::new("curl")
                                .args(["-s", "--max-time", "2", &probe])
                                .output()
                                .await
                            {
                                let text = String::from_utf8_lossy(&out.stdout).to_string();
                                if out.status.success() && text.contains("\"id\"") {
                                    found.push((
                                        format!("Endpoint :{port}"),
                                        format!("http://localhost:{port}/v1"),
                                    ));
                                }
                            }
                        }
                        found
                    },
                    |me, found, ctx| {
                        for (name, url) in found {
                            if !me.endpoints.iter().any(|e| e.base_url == url) {
                                let id = url
                                    .replace("http://", "")
                                    .replace('/', "_")
                                    .replace(':', "_");
                                me.endpoints.push(AiModelEntry::new_byoe(name, id, url));
                            }
                        }
                        me.save(ctx);
                        ctx.notify();
                    },
                );
            }
            AiProvidersPageAction::ShowAddForm => {
                self.show_add_form = true;
                ctx.notify();
            }
            AiProvidersPageAction::HideAddForm => {
                self.show_add_form = false;
                ctx.notify();
            }
            AiProvidersPageAction::SetAddPreset(url) => {
                self.add_preset_url = url.clone();
                let url_clone = url.clone();
                self.add_url_input.update(ctx, |i, ctx| {
                    i.editor()
                        .update(ctx, |ed, ctx| ed.set_buffer_text(&url_clone, ctx));
                });
                ctx.notify();
            }
            AiProvidersPageAction::AddEndpoint => {
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
                let name = name.trim().to_owned();
                if !name.is_empty() {
                    let id = format!(
                        "byoe-{}",
                        name.to_lowercase()
                            .replace(' ', "-")
                            .chars()
                            .take(24)
                            .collect::<String>()
                    );
                    self.endpoints
                        .push(AiModelEntry::new_byoe(name, id, url.trim()));
                    self.show_add_form = false;
                    for input in [
                        &self.add_name_input,
                        &self.add_url_input,
                        &self.add_key_input,
                    ] {
                        input.update(ctx, |i, ctx| {
                            i.editor().update(ctx, |ed, ctx| ed.clear_buffer(ctx))
                        });
                    }
                    self.save(ctx);
                }
                ctx.notify();
            }
            AiProvidersPageAction::ByoeToggleExpand(idx) => {
                let idx = *idx;
                self.endpoint_expanded = if self.endpoint_expanded == Some(idx) {
                    None
                } else {
                    self.populate_byoe_edit(idx, ctx);
                    Some(idx)
                };
                ctx.notify();
            }
            AiProvidersPageAction::ByoeToggleEnabled(idx) => {
                if let Some(m) = self.endpoints.get_mut(*idx) {
                    m.enabled = !m.enabled;
                    self.save(ctx);
                }
                ctx.notify();
            }
            AiProvidersPageAction::ByoeTestProvider(idx) => {
                let idx = *idx;
                self.endpoint_testing = Some(idx);
                ctx.notify();
                self.spawn_test_provider(idx, false, ctx);
            }
            AiProvidersPageAction::ByoeDeleteProvider(idx) => {
                let idx = *idx;
                if idx < self.endpoints.len() {
                    self.endpoints.remove(idx);
                    self.endpoint_expanded = match self.endpoint_expanded {
                        Some(ei) if ei == idx => None,
                        Some(ei) if ei > idx => Some(ei - 1),
                        other => other,
                    };
                    self.save(ctx);
                }
                ctx.notify();
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

// ── SettingsPageMeta ──────────────────────────────────────────────────────────

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
        "ai providers models llm gpt claude gemini openai anthropic google mistral cohere xai grok \
         ollama pull vram byoe endpoint vllm lmstudio kobold custom cloud scan detect discover api key"
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
        let err_col = theme.ui_error_color();

        // Page header
        let page_header = Flex::row()
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
            .with_child(ChildView::new(&view.sync_intel_button).finish())
            .finish();

        let page_desc = Container::new(
            Text::new(
                "Manage AI providers in three sections. Keys stored in ~/.specsmith/providers.json."
                    .to_string(),
                font,
                CONTENT_FONT_SIZE,
            )
            .with_color(sub)
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_top(4.)
        .with_margin_bottom(20.)
        .finish();

        // ═══════════════════════════════════════════════════════════════════
        // SECTION 1 — CLOUD PROVIDERS
        // ═══════════════════════════════════════════════════════════════════
        let cloud_hdr = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Expanded::new(
                    1.,
                    section_title(
                        "\u{2601}  Cloud Providers",
                        font,
                        active.into(),
                        CONTENT_FONT_SIZE,
                    ),
                )
                .finish(),
            )
            .with_child(if view.cloud_scanning {
                Text::new_inline("scanning\u{2026}", font, CONTENT_FONT_SIZE - 1.)
                    .with_color(sub)
                    .finish()
            } else {
                ChildView::new(&view.scan_all_button).finish()
            })
            .finish();

        let mut cloud_cards = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(4.);
        for (idx, model) in view.cloud_providers.iter().enumerate() {
            cloud_cards.add_child(render_provider_card(
                idx,
                model,
                view.cloud_expanded == Some(idx),
                view.cloud_testing == Some(idx),
                true,
                view,
                appearance,
                font,
                mono,
                sub,
                active.into(),
                accent,
                border,
                err_col,
            ));
        }

        // ═══════════════════════════════════════════════════════════════════
        // SECTION 2 — OLLAMA
        // ═══════════════════════════════════════════════════════════════════
        let ollama_hdr = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Expanded::new(
                    1.,
                    section_title("\u{1F999}  Ollama", font, active.into(), CONTENT_FONT_SIZE),
                )
                .finish(),
            )
            .with_child(ChildView::new(&view.detect_ollama_button).finish())
            .finish();

        let ollama_body = render_ollama_section(
            view,
            appearance,
            font,
            mono,
            sub,
            active.into(),
            accent,
            border,
            err_col,
        );

        // ═══════════════════════════════════════════════════════════════════
        // SECTION 3 — BYOE
        // ═══════════════════════════════════════════════════════════════════
        let byoe_hdr = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Expanded::new(
                    1.,
                    section_title(
                        "\u{1F50C}  Custom Endpoints (BYOE)",
                        font,
                        active.into(),
                        CONTENT_FONT_SIZE,
                    ),
                )
                .finish(),
            )
            .with_child(
                Container::new(ChildView::new(&view.discover_button).finish())
                    .with_margin_right(6.)
                    .finish(),
            )
            .with_child(ChildView::new(&view.add_endpoint_button).finish())
            .finish();

        let add_form_elem: Option<Box<dyn Element>> = if view.show_add_form {
            Some(render_byoe_add_form(
                view, appearance, font, accent, sub, border,
            ))
        } else {
            None
        };

        let mut byoe_cards = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(4.);
        for (idx, ep) in view.endpoints.iter().enumerate() {
            byoe_cards.add_child(render_provider_card(
                idx,
                ep,
                view.endpoint_expanded == Some(idx),
                view.endpoint_testing == Some(idx),
                false,
                view,
                appearance,
                font,
                mono,
                sub,
                active.into(),
                accent,
                border,
                err_col,
            ));
        }
        if view.endpoints.is_empty() && !view.show_add_form {
            byoe_cards.add_child(
                Container::new(
                    Text::new(
                        "No custom endpoints \u{2014} click \u{201c}+ Add Endpoint\u{201d} or \
                         \u{201c}\u{1F50E} Discover\u{201d} to scan local ports."
                            .to_string(),
                        font,
                        CONTENT_FONT_SIZE,
                    )
                    .with_color(sub)
                    .soft_wrap(true)
                    .finish(),
                )
                .with_uniform_padding(16.)
                .with_border(Border::all(1.).with_border_color(border))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
                .finish(),
            );
        }

        // ── Assemble ──────────────────────────────────────────────────────
        let mut page = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(page_header)
            .with_child(page_desc)
            .with_child(Container::new(cloud_hdr).with_margin_bottom(8.).finish())
            .with_child(cloud_cards.finish())
            .with_child(section_divider())
            .with_child(Container::new(ollama_hdr).with_margin_bottom(8.).finish())
            .with_child(ollama_body)
            .with_child(section_divider())
            .with_child(Container::new(byoe_hdr).with_margin_bottom(8.).finish());

        if let Some(form) = add_form_elem {
            page.add_child(form);
        }
        page.add_child(byoe_cards.finish());

        Container::new(
            ConstrainedBox::new(page.finish())
                .with_max_width(720.)
                .finish(),
        )
        .with_uniform_padding(28.)
        .finish()
    }
}

// ── Ollama section renderer ───────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_ollama_section(
    view: &AiProvidersPageView,
    appearance: &Appearance,
    font: warpui::fonts::FamilyId,
    mono: warpui::fonts::FamilyId,
    sub: pathfinder_color::ColorU,
    active: pathfinder_color::ColorU,
    accent: pathfinder_color::ColorU,
    border: pathfinder_color::ColorU,
    err_col: pathfinder_color::ColorU,
) -> Box<dyn Element> {
    let surface = appearance.theme().surface_1();
    let status_box = |msg: &str, col: pathfinder_color::ColorU| -> Box<dyn Element> {
        Container::new(
            Text::new(msg.to_owned(), font, CONTENT_FONT_SIZE)
                .with_color(col)
                .finish(),
        )
        .with_uniform_padding(14.)
        .with_border(Border::all(1.).with_border_color(border))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
        .finish()
    };

    match &view.ollama {
        OllamaSectionStatus::Idle => status_box(
            "\u{25CB} Not detected \u{2014} click \u{201c}\u{1F999} Detect Ollama\u{201d} to scan.",
            sub,
        ),
        OllamaSectionStatus::Detecting => status_box("Detecting Ollama\u{2026}", sub),
        OllamaSectionStatus::Offline => status_box(
            "\u{25CF} Offline \u{2014} Ollama not found on ports 11434\u{2013}11436.",
            err_col,
        ),
        OllamaSectionStatus::Pulling {
            model_name,
            progress,
        } => status_box(&format!("Pulling {model_name}\u{2026}  {progress}"), sub),
        OllamaSectionStatus::PullDone { model_name } => status_box(
            &format!("\u{2714}  {model_name} pulled successfully."),
            accent,
        ),
        OllamaSectionStatus::PullError { model_name, error } => status_box(
            &format!("\u{2717}  Pull failed ({model_name}): {error}"),
            err_col,
        ),
        OllamaSectionStatus::Deleting { model_name } => {
            status_box(&format!("Deleting {model_name}\u{2026}"), sub)
        }
        OllamaSectionStatus::Online { url, models } => {
            // Status row
            let status_row = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Container::new(
                        Text::new_inline("\u{25CF}", font, 13.)
                            .with_color(accent.into())
                            .finish(),
                    )
                    .with_margin_right(6.)
                    .finish(),
                )
                .with_child(
                    Text::new_inline(
                        format!("Online  \u{2014}  {url}  ({} model(s))", models.len()),
                        font,
                        CONTENT_FONT_SIZE,
                    )
                    .with_color(active)
                    .finish(),
                )
                .finish();

            // VRAM filter chips
            let vram_chip = |gb: u32, label: &'static str| -> Box<dyn Element> {
                let selected = view.vram_filter_gb == gb;
                let bg = if selected { accent } else { sub };
                Hoverable::new(MouseStateHandle::default(), move |_| {
                    Container::new(
                        Text::new_inline(label.to_owned(), font, CONTENT_FONT_SIZE - 2.)
                            .with_color(pathfinder_color::ColorU::white())
                            .finish(),
                    )
                    .with_background_color(bg)
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(3.)))
                    .with_horizontal_padding(7.)
                    .with_vertical_padding(3.)
                    .finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(AiProvidersPageAction::SetVramFilter(gb))
                })
                .finish()
            };

            let vram_row = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Container::new(
                        Text::new_inline("VRAM filter:", font, CONTENT_FONT_SIZE - 1.)
                            .with_color(sub)
                            .finish(),
                    )
                    .with_margin_right(6.)
                    .finish(),
                )
                .with_spacing(4.)
                .with_child(vram_chip(0, "All"))
                .with_child(vram_chip(8, "\u{2264}8 GB"))
                .with_child(vram_chip(16, "\u{2264}16 GB"))
                .with_child(vram_chip(32, "\u{2264}32 GB"))
                .finish();

            // Model list
            let filtered: Vec<&OllamaModel> = models
                .iter()
                .filter(|m| {
                    view.vram_filter_gb == 0 || m.vram_gb_estimate() <= view.vram_filter_gb as f32
                })
                .collect();

            let mut model_list = Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_spacing(1.);

            if filtered.is_empty() && !models.is_empty() {
                model_list.add_child(
                    Text::new_inline(
                        "No models fit the VRAM filter.".to_string(),
                        font,
                        CONTENT_FONT_SIZE - 1.,
                    )
                    .with_color(sub)
                    .finish(),
                );
            }
            for m in &filtered {
                let model_name = m.name.clone();
                let size_str = m.size_display();
                let param_str = if m.param_size.is_empty() {
                    String::new()
                } else {
                    format!("  {}", m.param_size)
                };
                let vram_str = format!("  ~{:.0} GB VRAM", m.vram_gb_estimate());
                let del_name = model_name.clone();

                model_list.add_child(
                    Container::new(
                        Flex::row()
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_child(
                                Expanded::new(
                                    1.,
                                    Flex::row()
                                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                        .with_spacing(6.)
                                        .with_child(
                                            Text::new_inline(
                                                model_name.clone(),
                                                mono,
                                                CONTENT_FONT_SIZE - 1.,
                                            )
                                            .with_color(active)
                                            .finish(),
                                        )
                                        .with_child(
                                            Text::new_inline(
                                                format!("{size_str}{param_str}{vram_str}"),
                                                font,
                                                CONTENT_FONT_SIZE - 2.,
                                            )
                                            .with_color(sub)
                                            .finish(),
                                        )
                                        .finish(),
                                )
                                .finish(),
                            )
                            .with_child(
                                Hoverable::new(MouseStateHandle::default(), move |_| {
                                    Text::new_inline(
                                        "\u{1F5D1} delete".to_string(),
                                        font,
                                        CONTENT_FONT_SIZE - 2.,
                                    )
                                    .with_color(err_col.into())
                                    .finish()
                                })
                                .with_cursor(Cursor::PointingHand)
                                .on_click(move |ctx, _, _| {
                                    ctx.dispatch_typed_action(
                                        AiProvidersPageAction::DeleteOllamaModel(del_name.clone()),
                                    )
                                })
                                .finish(),
                            )
                            .finish(),
                    )
                    .with_vertical_padding(5.)
                    .with_border(Border::bottom(1.).with_border_color(border))
                    .finish(),
                );
            }

            // Pull form
            let pull_row = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(6.)
                .with_child(
                    Expanded::new(1., ChildView::new(&view.pull_model_input).finish()).finish(),
                )
                .with_child(
                    Hoverable::new(MouseStateHandle::default(), move |_| {
                        Container::new(
                            Text::new_inline(
                                "\u{2B07} Pull".to_string(),
                                font,
                                CONTENT_FONT_SIZE - 1.,
                            )
                            .with_color(accent.into())
                            .finish(),
                        )
                        .with_horizontal_padding(8.)
                        .with_vertical_padding(4.)
                        .with_border(Border::all(1.).with_border_color(accent))
                        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
                        .finish()
                    })
                    .with_cursor(Cursor::PointingHand)
                    .on_click(|ctx, _, _| {
                        ctx.dispatch_typed_action(AiProvidersPageAction::PullOllamaModel)
                    })
                    .finish(),
                )
                .finish();

            Container::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(10.)
                    .with_child(status_row)
                    .with_child(Container::new(vram_row).with_margin_top(2.).finish())
                    .with_child(
                        Container::new(model_list.finish())
                            .with_margin_top(4.)
                            .finish(),
                    )
                    .with_child(Container::new(pull_row).with_margin_top(6.).finish())
                    .finish(),
            )
            .with_background(surface)
            .with_border(Border::all(1.).with_border_color(border))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
            .with_uniform_padding(12.)
            .finish()
        }
    }
}

// ── BYOE add form ─────────────────────────────────────────────────────────────

fn render_byoe_add_form(
    view: &AiProvidersPageView,
    appearance: &Appearance,
    font: warpui::fonts::FamilyId,
    accent: pathfinder_color::ColorU,
    sub: pathfinder_color::ColorU,
    border: pathfinder_color::ColorU,
) -> Box<dyn Element> {
    let surface = appearance.theme().surface_1();
    let mut presets_row = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(4.);
    for (label, url) in BYOE_PRESETS {
        let selected = view.add_preset_url.as_str() == *url;
        let bg = if selected { accent } else { sub };
        let url_s = url.to_string();
        presets_row.add_child(
            Hoverable::new(MouseStateHandle::default(), move |_| {
                Container::new(
                    Text::new_inline(label.to_string(), font, CONTENT_FONT_SIZE - 2.)
                        .with_color(pathfinder_color::ColorU::white())
                        .finish(),
                )
                .with_background_color(bg)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(3.)))
                .with_horizontal_padding(7.)
                .with_vertical_padding(3.)
                .finish()
            })
            .with_cursor(Cursor::PointingHand)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(AiProvidersPageAction::SetAddPreset(url_s.clone()))
            })
            .finish(),
        );
    }

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
        .on_click(|ctx, _, _| ctx.dispatch_typed_action(AiProvidersPageAction::AddEndpoint))
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

    Container::new(
        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(6.)
            .with_child(
                Text::new(
                    "Add Custom Endpoint".to_string(),
                    font,
                    CONTENT_FONT_SIZE + 1.,
                )
                .with_style(Properties::default().weight(Weight::Semibold))
                .with_color(appearance.theme().active_ui_text_color().into())
                .finish(),
            )
            .with_child(
                Container::new(presets_row.finish())
                    .with_margin_top(2.)
                    .finish(),
            )
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
    .with_margin_bottom(10.)
    .finish()
}

// ── Shared provider card ──────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_provider_card(
    idx: usize,
    model: &AiModelEntry,
    is_exp: bool,
    is_test: bool,
    is_cloud: bool,
    view: &AiProvidersPageView,
    appearance: &Appearance,
    font: warpui::fonts::FamilyId,
    mono: warpui::fonts::FamilyId,
    sub: pathfinder_color::ColorU,
    active: pathfinder_color::ColorU,
    accent: pathfinder_color::ColorU,
    border: pathfinder_color::ColorU,
    err_col: pathfinder_color::ColorU,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let surface = theme.surface_1();
    let surface_color = theme.surface_1().into_solid();
    let hover_bg = internal_colors::neutral_1(theme);
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
    let url_preview: String = if model.base_url.len() > 48 {
        format!("{}…", &model.base_url[..45])
    } else {
        model.base_url.clone()
    };
    let key_hint = if model.api_key_set {
        "\u{1F511}"
    } else {
        "\u{2205}"
    };
    let enabled = model.enabled;

    let toggle_action = if is_cloud {
        AiProvidersPageAction::CloudToggleEnabled(idx)
    } else {
        AiProvidersPageAction::ByoeToggleEnabled(idx)
    };
    let test_action = if is_cloud {
        AiProvidersPageAction::CloudTestProvider(idx)
    } else {
        AiProvidersPageAction::ByoeTestProvider(idx)
    };
    let expand_action = if is_cloud {
        AiProvidersPageAction::CloudToggleExpand(idx)
    } else {
        AiProvidersPageAction::ByoeToggleExpand(idx)
    };

    let header_elem = Hoverable::new(model.row_hover.clone(), move |state| {
        let bg = if state.is_hovered() {
            hover_bg
        } else {
            surface_color
        };
        Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Container::new(Text::new_inline(icon_str.clone(), font, 16.).finish())
                        .with_margin_right(8.)
                        .finish(),
                )
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
                                        .with_style(Properties::default().weight(Weight::Semibold))
                                        .with_color(active)
                                        .finish(),
                                    )
                                    .with_child(badge(
                                        &type_lbl,
                                        accent,
                                        font,
                                        CONTENT_FONT_SIZE - 1.,
                                    ))
                                    .with_child(
                                        Text::new_inline(
                                            key_hint.to_string(),
                                            font,
                                            CONTENT_FONT_SIZE - 1.,
                                        )
                                        .with_color(
                                            if model.api_key_set { accent } else { sub }.into(),
                                        )
                                        .finish(),
                                    )
                                    .finish(),
                            )
                            .with_child(
                                Flex::row()
                                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                    .with_spacing(8.)
                                    .with_child(
                                        Text::new_inline(
                                            stat_lbl.clone(),
                                            font,
                                            CONTENT_FONT_SIZE - 1.,
                                        )
                                        .with_color(stat_col)
                                        .finish(),
                                    )
                                    .with_child(if url_preview.is_empty() {
                                        warpui::elements::Empty::new().finish()
                                    } else {
                                        Text::new_inline(
                                            url_preview.clone(),
                                            mono,
                                            CONTENT_FONT_SIZE - 2.,
                                        )
                                        .with_color(sub)
                                        .finish()
                                    })
                                    .finish(),
                            )
                            .finish(),
                    )
                    .finish(),
                )
                .with_child(
                    Container::new(
                        Hoverable::new(MouseStateHandle::default(), move |ts| {
                            let lbl = if enabled { "[on]" } else { "[off]" };
                            let col = if ts.is_hovered() {
                                active
                            } else if enabled {
                                accent.into()
                            } else {
                                sub.into()
                            };
                            Text::new_inline(lbl.to_string(), font, CONTENT_FONT_SIZE - 1.)
                                .with_color(col)
                                .finish()
                        })
                        .with_cursor(Cursor::PointingHand)
                        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(toggle_action.clone()))
                        .finish(),
                    )
                    .with_margin_left(6.)
                    .with_margin_right(6.)
                    .finish(),
                )
                .with_child(
                    Container::new(
                        Hoverable::new(MouseStateHandle::default(), move |ts| {
                            let lbl = if is_test { "\u{2026}" } else { "Test" };
                            let col = if ts.is_hovered() { active } else { sub.into() };
                            Text::new_inline(lbl.to_string(), font, CONTENT_FONT_SIZE - 1.)
                                .with_color(col)
                                .finish()
                        })
                        .with_cursor(Cursor::PointingHand)
                        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(test_action.clone()))
                        .finish(),
                    )
                    .with_margin_right(6.)
                    .finish(),
                )
                .with_child(
                    Text::new_inline(if is_exp { "\u{25B2}" } else { "\u{25BC}" }, font, 9.)
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
    .on_click(move |ctx, _, _| ctx.dispatch_typed_action(expand_action.clone()))
    .finish();

    // Expanded detail panel
    let detail_elem: Option<Box<dyn Element>> = if is_exp {
        let action_btn = if is_cloud {
            // Cloud: disable (not deletable from catalog)
            None
        } else {
            let del_btn = appearance
                .ui_builder()
                .button(ButtonVariant::Secondary, MouseStateHandle::default())
                .with_style(UiComponentStyles {
                    font_size: Some(CONTENT_FONT_SIZE - 1.),
                    padding: Some(Coords::uniform(5.)),
                    ..Default::default()
                })
                .with_centered_text_label("Delete Endpoint".to_string())
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(AiProvidersPageAction::ByoeDeleteProvider(idx))
                })
                .finish();
            Some(del_btn)
        };

        let key_input = if is_cloud {
            &view.cloud_edit_key_input
        } else {
            &view.byoe_edit_key_input
        };
        let url_input = if is_cloud {
            &view.cloud_edit_url_input
        } else {
            &view.byoe_edit_url_input
        };

        let mut detail = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(4.);

        if !is_cloud {
            detail.add_child(labeled_input(
                "Name",
                &view.byoe_edit_name_input,
                font,
                sub,
                CONTENT_FONT_SIZE,
            ));
        }
        detail.add_child(labeled_input(
            "Base URL",
            url_input,
            font,
            sub,
            CONTENT_FONT_SIZE,
        ));
        detail.add_child(labeled_input(
            "API Key",
            key_input,
            font,
            sub,
            CONTENT_FONT_SIZE,
        ));

        if !model.available_models.is_empty() {
            let mut chips = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(4.);
            for m in model.available_models.iter().take(10) {
                chips.add_child(
                    Container::new(
                        Text::new_inline(m.clone(), mono, CONTENT_FONT_SIZE - 2.)
                            .with_color(active)
                            .finish(),
                    )
                    .with_background_color(internal_colors::neutral_2(appearance.theme()))
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(3.)))
                    .with_horizontal_padding(5.)
                    .with_vertical_padding(2.)
                    .finish(),
                );
            }
            if model.available_models.len() > 10 {
                chips.add_child(
                    Text::new_inline(
                        format!("+{} more", model.available_models.len() - 10),
                        font,
                        CONTENT_FONT_SIZE - 2.,
                    )
                    .with_color(sub)
                    .finish(),
                );
            }
            detail.add_child(Container::new(chips.finish()).with_margin_top(4.).finish());
        }

        if let Some(btn) = action_btn {
            detail.add_child(Container::new(btn).with_margin_top(6.).finish());
        }

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

    let mut card_col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
    card_col.add_child(header_elem);
    if let Some(d) = detail_elem {
        card_col.add_child(d);
    }

    Container::new(card_col.finish())
        .with_background(surface)
        .with_border(Border::all(1.).with_border_color(border))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
        .finish()
}

// ── UI helpers ────────────────────────────────────────────────────────────────

fn section_title(
    text: &str,
    font: warpui::fonts::FamilyId,
    color: pathfinder_color::ColorU,
    font_size: f32,
) -> Box<dyn Element> {
    Text::new(text.to_owned(), font, font_size + 2.)
        .with_style(Properties::default().weight(Weight::Semibold))
        .with_color(color.into())
        .finish()
}

fn section_divider() -> Box<dyn Element> {
    Container::new(warpui::elements::Empty::new().finish())
        .with_margin_top(16.)
        .with_margin_bottom(16.)
        .finish()
}

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
