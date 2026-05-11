//! AI Providers settings page.
//!
//! Shows and manages AI model providers available to the Kairos agent.
//! Models are persisted to `~/.specsmith/providers.json` and loaded on startup.

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
use warp_core::ui::theme::color::internal_colors;
use warpui::{
    elements::{
        Border, ChildView, Clipped, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment,
        Element, Flex, MouseStateHandle, ParentElement, Radius, Text,
    },
    fonts::{Properties, Weight},
    platform::Cursor,
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

// ── Column widths ─────────────────────────────────────────────────────────────

const NAME_COL_MAX_WIDTH: f32 = 200.;
const ID_COL_MAX_WIDTH: f32 = 220.;
const TOKEN_COL_WIDTH: f32 = 80.;
const ROW_HEIGHT: f32 = 32.;
const CELL_PADDING_H: f32 = 8.;

// ── Persistence ───────────────────────────────────────────────────────────────

/// Return the path to `~/.specsmith/providers.json`.
fn providers_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".specsmith").join("providers.json"))
}

/// Load models from `~/.specsmith/providers.json`.  Returns `None` on any error.
fn load_providers() -> Option<Vec<AiModelEntry>> {
    let path = providers_path()?;
    let text = std::fs::read_to_string(&path).ok()?;
    let arr: Vec<serde_json::Value> = serde_json::from_str(&text).ok()?;
    Some(
        arr.into_iter()
            .filter_map(|v| {
                Some(AiModelEntry::new(
                    v["name"].as_str()?,
                    v["id"].as_str()?,
                    v["context_tokens"].as_u64(),
                    v["output_tokens"].as_u64(),
                ))
            })
            .collect(),
    )
}

/// Serialize `models` to JSON and return the bytes.
fn serialize_providers(models: &[AiModelEntry]) -> String {
    let arr: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            let mut obj = serde_json::json!({
                "name": m.name,
                "id": m.id,
            });
            if let Some(ctx) = m.context_tokens {
                obj["context_tokens"] = serde_json::Value::from(ctx);
            }
            if let Some(out) = m.output_tokens {
                obj["output_tokens"] = serde_json::Value::from(out);
            }
            obj
        })
        .collect();
    serde_json::to_string_pretty(&arr).unwrap_or_default()
}

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct AiModelEntry {
    pub name: String,
    pub id: String,
    pub context_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub row_hover: MouseStateHandle,
}

impl AiModelEntry {
    pub fn new(
        name: impl Into<String>,
        id: impl Into<String>,
        context_tokens: Option<u64>,
        output_tokens: Option<u64>,
    ) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
            context_tokens,
            output_tokens,
            row_hover: MouseStateHandle::default(),
        }
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AiProvidersPageAction {
    AddModel,
    RemoveSelected,
    FetchFromApi,
    SyncFromModelsDev,
    SelectModel(usize),
}

// ── View ──────────────────────────────────────────────────────────────────────

pub struct AiProvidersPageView {
    page: PageType<Self>,
    pub models: Vec<AiModelEntry>,
    pub selected_index: Option<usize>,
    add_button: ViewHandle<ActionButton>,
    remove_button: ViewHandle<ActionButton>,
    fetch_button: ViewHandle<ActionButton>,
    sync_button: ViewHandle<ActionButton>,
}

impl AiProvidersPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let add_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("+ Add model", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::AddModel))
        });
        let remove_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Remove", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::RemoveSelected))
        });
        let fetch_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Fetch from API", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::FetchFromApi))
        });
        let sync_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Sync from models.dev", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(AiProvidersPageAction::SyncFromModelsDev))
        });

        // Load persisted models; fall back to well-known defaults.
        let models = load_providers().unwrap_or_else(|| {
            vec![
                AiModelEntry::new("GPT-5.5", "gpt-5.5", Some(1_050_000), Some(128_000)),
                AiModelEntry::new("GPT-5.5 Pro", "gpt-5.5-pro", Some(1_050_000), Some(128_000)),
                AiModelEntry::new("GPT-4.1", "gpt-4.1", Some(1_047_576), Some(32_768)),
                AiModelEntry::new("o3", "o3", Some(200_000), Some(100_000)),
                AiModelEntry::new("o3-mini", "o3-mini", Some(128_000), Some(65_536)),
                AiModelEntry::new("o4-mini", "o4-mini", Some(200_000), Some(100_000)),
                AiModelEntry::new(
                    "o4-mini-deep-research",
                    "o4-mini-deep-research",
                    Some(200_000),
                    Some(100_000),
                ),
                AiModelEntry::new("o1", "o1", Some(200_000), Some(100_000)),
                AiModelEntry::new("o1-mini", "o1-mini", Some(128_000), Some(65_536)),
            ]
        });

        Self {
            page: PageType::new_monolith(AiProvidersPageWidget, None, false),
            models,
            selected_index: None,
            add_button,
            remove_button,
            fetch_button,
            sync_button,
        }
    }

    /// Persist the current model list to `~/.specsmith/providers.json`.
    fn save(&self, ctx: &mut ViewContext<Self>) {
        #[cfg(not(target_family = "wasm"))]
        {
            let json = serialize_providers(&self.models);
            ctx.spawn(
                async move {
                    if let Some(path) = providers_path() {
                        if let Some(parent) = path.parent() {
                            let _ = tokio::fs::create_dir_all(parent).await;
                        }
                        let _ = tokio::fs::write(&path, json).await;
                    }
                },
                |_, _, _| {},
            );
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
            AiProvidersPageAction::SelectModel(idx) => {
                let idx = *idx;
                if idx < self.models.len() {
                    self.selected_index = if self.selected_index == Some(idx) {
                        None
                    } else {
                        Some(idx)
                    };
                    ctx.notify();
                }
            }
            AiProvidersPageAction::RemoveSelected => {
                if let Some(idx) = self.selected_index.take() {
                    if idx < self.models.len() {
                        self.models.remove(idx);
                        self.save(ctx);
                    }
                    ctx.notify();
                }
            }
            AiProvidersPageAction::AddModel => {
                // Add a placeholder that the user can identify and edit.
                let placeholder = AiModelEntry::new(
                    "New Model",
                    &format!("new-model-{}", self.models.len() + 1),
                    None,
                    None,
                );
                self.models.push(placeholder);
                self.selected_index = Some(self.models.len() - 1);
                self.save(ctx);
                ctx.notify();
            }
            AiProvidersPageAction::FetchFromApi => {
                // Spawn `specsmith agent providers --json` and populate the list.
                #[cfg(not(target_family = "wasm"))]
                ctx.spawn(
                    async move {
                        tokio::process::Command::new("specsmith")
                            .args(["agent", "providers", "--json"])
                            .output()
                            .await
                    },
                    |me, result, ctx| {
                        if let Ok(output) = result {
                            if output.status.success() {
                                let text = String::from_utf8_lossy(&output.stdout).to_string();
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(providers) = v["providers"].as_array() {
                                        let new_entries: Vec<AiModelEntry> = providers
                                            .iter()
                                            .filter_map(|p| {
                                                let name = p["name"].as_str()?;
                                                let id = p["name"].as_str()?;
                                                Some(AiModelEntry::new(name, id, None, None))
                                            })
                                            .collect();
                                        if !new_entries.is_empty() {
                                            me.models = new_entries;
                                            me.save(ctx);
                                            ctx.notify();
                                        }
                                    }
                                }
                            }
                        }
                    },
                );
            }
            AiProvidersPageAction::SyncFromModelsDev => {
                // Fetch common model IDs from the models.dev public API and
                // merge into the current list (existing entries are preserved).
                #[cfg(not(target_family = "wasm"))]
                ctx.spawn(
                    async move {
                        tokio::process::Command::new("specsmith")
                            .args(["ollama", "available", "--json"])
                            .output()
                            .await
                    },
                    |me, result, ctx| {
                        if let Ok(output) = result {
                            if output.status.success() {
                                let text = String::from_utf8_lossy(&output.stdout).to_string();
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(models) = v["models"].as_array() {
                                        let existing_ids: std::collections::HashSet<String> =
                                            me.models.iter().map(|m| m.id.clone()).collect();
                                        for m in models {
                                            let id = match m["id"].as_str() {
                                                Some(s) => s.to_string(),
                                                None => continue,
                                            };
                                            if !existing_ids.contains(&id) {
                                                let name =
                                                    m["name"].as_str().unwrap_or(&id).to_string();
                                                me.models
                                                    .push(AiModelEntry::new(name, id, None, None));
                                            }
                                        }
                                        me.save(ctx);
                                        ctx.notify();
                                    }
                                }
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

impl From<ViewHandle<AiProvidersPageView>> for SettingsPageViewHandle {
    fn from(handle: ViewHandle<AiProvidersPageView>) -> Self {
        SettingsPageViewHandle::AiProviders(handle)
    }
}

// ── Widget ────────────────────────────────────────────────────────────────────

struct AiProvidersPageWidget;

impl SettingsWidget for AiProvidersPageWidget {
    type View = AiProvidersPageView;

    fn search_terms(&self) -> &str {
        "providers models ai llm gpt claude gemini openai anthropic endpoint tokens context output"
    }

    fn render(
        &self,
        view: &AiProvidersPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let font = appearance.ui_font_family();
        let mono_font = appearance.monospace_font_family();
        let sub_color = blended_colors::text_sub(theme, theme.surface_1());
        let active_color = theme.active_ui_text_color().into();
        let border_color = internal_colors::neutral_2(theme);
        let row_hover_bg = internal_colors::neutral_1(theme);
        // accent_overlay_1 and surface_1 return warp_core::ui::theme::Fill;
        // call into_solid() to get ColorU so all three bg variants have the same type.
        let selected_bg = internal_colors::accent_overlay_1(theme).into_solid();
        let surface_color = theme.surface_1().into_solid();

        // ── Page header ───────────────────────────────────────────────
        let page_header = Container::new(
            Text::new(
                "AI Model Providers".to_string(),
                font,
                CONTENT_FONT_SIZE + 4.,
            )
            .with_style(Properties::default().weight(Weight::Semibold))
            .with_color(active_color)
            .finish(),
        )
        .with_margin_bottom(4.)
        .finish();

        let page_desc = Container::new(
            Text::new(
                "Manage AI model endpoints. Models are saved to ~/.specsmith/providers.json."
                    .to_string(),
                font,
                CONTENT_FONT_SIZE,
            )
            .with_color(sub_color)
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_bottom(16.)
        .finish();

        // ── Table header row ──────────────────────────────────────────
        let table_header = Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    ConstrainedBox::new(
                        Text::new("NAME".to_string(), font, CONTENT_FONT_SIZE - 1.)
                            .with_color(sub_color)
                            .with_style(Properties::default().weight(Weight::Semibold))
                            .finish(),
                    )
                    .with_width(NAME_COL_MAX_WIDTH)
                    .finish(),
                )
                .with_child(
                    ConstrainedBox::new(
                        Text::new("MODEL ID".to_string(), font, CONTENT_FONT_SIZE - 1.)
                            .with_color(sub_color)
                            .with_style(Properties::default().weight(Weight::Semibold))
                            .finish(),
                    )
                    .with_width(ID_COL_MAX_WIDTH)
                    .finish(),
                )
                .with_child(
                    ConstrainedBox::new(
                        Text::new("CONTEXT".to_string(), font, CONTENT_FONT_SIZE - 1.)
                            .with_color(sub_color)
                            .with_style(Properties::default().weight(Weight::Semibold))
                            .finish(),
                    )
                    .with_width(TOKEN_COL_WIDTH)
                    .finish(),
                )
                .with_child(
                    ConstrainedBox::new(
                        Text::new("OUTPUT".to_string(), font, CONTENT_FONT_SIZE - 1.)
                            .with_color(sub_color)
                            .with_style(Properties::default().weight(Weight::Semibold))
                            .finish(),
                    )
                    .with_width(TOKEN_COL_WIDTH)
                    .finish(),
                )
                .finish(),
        )
        .with_padding_left(CELL_PADDING_H)
        .with_padding_right(CELL_PADDING_H)
        .with_border(Border::bottom(1.).with_border_color(border_color))
        .finish();

        // ── Model rows ────────────────────────────────────────────────
        let mut rows = Flex::column();
        for (idx, model) in view.models.iter().enumerate() {
            let is_selected = view.selected_index == Some(idx);
            let name = model.name.clone();
            let id = model.id.clone();
            let ctx_str = model
                .context_tokens
                .map(format_tokens)
                .unwrap_or_else(|| "\u{2014}".to_string());
            let out_str = model
                .output_tokens
                .map(format_tokens)
                .unwrap_or_else(|| "\u{2014}".to_string());

            let row_container =
                warpui::elements::Hoverable::new(model.row_hover.clone(), move |state| {
                    let bg = if is_selected {
                        selected_bg
                    } else if state.is_hovered() {
                        row_hover_bg
                    } else {
                        surface_color
                    };
                    let row = Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            ConstrainedBox::new(
                                Clipped::new(
                                    Text::new(name.clone(), font, CONTENT_FONT_SIZE)
                                        .with_color(active_color)
                                        .finish(),
                                )
                                .finish(),
                            )
                            .with_width(NAME_COL_MAX_WIDTH)
                            .finish(),
                        )
                        .with_child(
                            ConstrainedBox::new(
                                Clipped::new(
                                    Text::new(id.clone(), mono_font, CONTENT_FONT_SIZE - 1.)
                                        .with_color(sub_color)
                                        .finish(),
                                )
                                .finish(),
                            )
                            .with_width(ID_COL_MAX_WIDTH)
                            .finish(),
                        )
                        .with_child(
                            ConstrainedBox::new(
                                Text::new(ctx_str.clone(), font, CONTENT_FONT_SIZE - 1.)
                                    .with_color(sub_color)
                                    .finish(),
                            )
                            .with_width(TOKEN_COL_WIDTH)
                            .finish(),
                        )
                        .with_child(
                            ConstrainedBox::new(
                                Text::new(out_str.clone(), font, CONTENT_FONT_SIZE - 1.)
                                    .with_color(sub_color)
                                    .finish(),
                            )
                            .with_width(TOKEN_COL_WIDTH)
                            .finish(),
                        )
                        .finish();
                    ConstrainedBox::new(
                        Container::new(row)
                            .with_background_color(bg)
                            .with_padding_left(CELL_PADDING_H)
                            .with_padding_right(CELL_PADDING_H)
                            .finish(),
                    )
                    .with_height(ROW_HEIGHT)
                    .finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(AiProvidersPageAction::SelectModel(idx));
                })
                .finish();

            rows.add_child(row_container);
        }

        // ── Empty state ───────────────────────────────────────────────
        let table_body = if view.models.is_empty() {
            Container::new(
                Text::new(
                    "No models configured. Add a model or fetch from an API.".to_string(),
                    font,
                    CONTENT_FONT_SIZE,
                )
                .with_color(sub_color)
                .finish(),
            )
            .with_uniform_padding(16.)
            .finish()
        } else {
            rows.finish()
        };

        // ── Table border ──────────────────────────────────────────────
        let table = Container::new(
            Flex::column()
                .with_child(table_header)
                .with_child(table_body)
                .finish(),
        )
        .with_border(Border::all(1.).with_border_color(border_color))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
        .with_margin_bottom(12.)
        .finish();

        // ── Action bar ────────────────────────────────────────────────
        let action_bar = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(6.)
            .with_child(ChildView::new(&view.add_button).finish())
            .with_child(ChildView::new(&view.fetch_button).finish())
            .with_child(ChildView::new(&view.sync_button).finish())
            .with_child(ChildView::new(&view.remove_button).finish())
            .finish();

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(page_header)
            .with_child(page_desc)
            .with_child(table)
            .with_child(action_bar)
            .finish()
    }
}

/// Format a token count as a short string (e.g. 200_000 -> "200K", 1_048_576 -> "1M").
fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        let m = n as f64 / 1_000_000.0;
        if m == m.floor() {
            format!("{}M", m as u64)
        } else {
            format!("{:.1}M", m)
        }
    } else if n >= 1_000 {
        let k = n as f64 / 1_000.0;
        if k == k.floor() {
            format!("{}K", k as u64)
        } else {
            format!("{:.0}K", k)
        }
    } else {
        n.to_string()
    }
}
