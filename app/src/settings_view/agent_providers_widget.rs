//! Custom Agent Provider settings panel widget.
//!
//! UI layout:
//! - Sub-header (title left + top-right `+ Add provider` button) + short description
//! - Each provider has a card containing:
//!     · `Name` / `Base URL` / `API Key` three input fields (save on blur/Enter)
//!     · Model list area: header `Display Name | Model ID`, each row has two inputs + `×` remove button
//!     · Bottom button row: `+ Add model` `Fetch from API` `Remove` (provider)
//!
//! When the provider list size or a provider's models count changes,
//! `AISettingsPageView::rebuild_current_page` is triggered to rebuild the entire widget,
//! so newly added/removed entries get their own EditorView handle.
//! `rebuild_current_page` internally reuses the old PageType's vertical scroll handle,
//! so scroll position is not reset.
//!
//! Provider metadata (name/base_url/models) uses `settings.toml`,
//! `api_key` uses OS keychain (`AgentProviderSecrets`).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use settings::Setting;
use warpui::elements::{
    ChildView, Container, CornerRadius, CrossAxisAlignment, Expanded, Flex, MainAxisAlignment,
    MouseStateHandle, ParentElement, Radius, Text, Wrap,
};
use warpui::ui_components::{
    button::ButtonVariant,
    components::{Coords, UiComponent, UiComponentStyles},
};
use warpui::{AppContext, Element, SingletonEntity, ViewContext, ViewHandle};

use crate::ai::agent_providers::AgentProviderSecrets;
use crate::appearance::Appearance;
use crate::editor::{
    EditorView, Event as EditorEvent, SingleLineEditorOptions, TextColors, TextOptions,
};
use crate::settings::{AISettings, AgentProvider, AgentProviderApiType, AgentProviderModel};
use strum::IntoEnumIterator;

use super::ai_page::{AISettingsPageAction, AISettingsPageView, ModelCapabilityKind};
use super::settings_page::{build_sub_header, SettingsWidget, HEADER_PADDING};

const CARD_BUTTON_FONT_SIZE: f32 = 12.0;
const CARD_BUTTON_PADDING: f32 = 6.0;
const FIELD_LABEL_MARGIN_TOP: f32 = 6.0;
const FIELD_LABEL_MARGIN_BOTTOM: f32 = 2.0;
const MODEL_ROW_GAP: f32 = 6.0;

// ---------------------------------------------------------------------------
// Model row expand state (process-local, thread_local; UI-thread-safe; not persisted)
// ---------------------------------------------------------------------------

std::thread_local! {
    /// {provider_id => Set<model_index>} currently expanded model entries.
    /// Discarded when settings page is closed; similar to `models_dev::chips_expanded()` AtomicBool.
    static EXPANDED_MODELS: RefCell<HashMap<String, HashSet<usize>>> = RefCell::new(HashMap::new());
}

pub(super) fn is_model_expanded(provider_id: &str, model_index: usize) -> bool {
    EXPANDED_MODELS.with(|m| {
        m.borrow()
            .get(provider_id)
            .map_or(false, |set| set.contains(&model_index))
    })
}

pub(super) fn toggle_model_expanded(provider_id: &str, model_index: usize) {
    EXPANDED_MODELS.with(|m| {
        let mut map = m.borrow_mut();
        let set = map.entry(provider_id.to_string()).or_default();
        if !set.insert(model_index) {
            set.remove(&model_index);
        }
    });
}

/// When a provider is removed, clear its expand records to avoid index drift.
pub(super) fn clear_expanded_models_for_provider(provider_id: &str) {
    EXPANDED_MODELS.with(|m| {
        m.borrow_mut().remove(provider_id);
    });
}

/// Editable view handles for a single model row entry (name + id + context + output).
struct ModelRow {
    name_editor: ViewHandle<EditorView>,
    id_editor: ViewHandle<EditorView>,
    context_editor: ViewHandle<EditorView>,
    output_editor: ViewHandle<EditorView>,
    /// Remove button — hidden in collapsed state; appears at the end of the detail panel when expanded.
    remove_button_state: MouseStateHandle,
    /// Expand/collapse chevron at the end of the row.
    expand_button_state: MouseStateHandle,
    /// Mouse state for the image/pdf/audio three-state chips in the detail panel.
    image_chip_state: MouseStateHandle,
    pdf_chip_state: MouseStateHandle,
    audio_chip_state: MouseStateHandle,
    /// Mouse state for the reasoning / tool_call bool toggles in the detail panel.
    reasoning_chip_state: MouseStateHandle,
    tool_call_chip_state: MouseStateHandle,
}

struct HeaderRow {
    key_editor: ViewHandle<EditorView>,
    val_editor: ViewHandle<EditorView>,
    remove_button_state: MouseStateHandle,
}

/// All editable view handles for a single provider row.
struct ProviderRow {
    name_editor: ViewHandle<EditorView>,
    base_url_editor: ViewHandle<EditorView>,
    api_key_editor: ViewHandle<EditorView>,
    fetch_button_state: MouseStateHandle,
    sync_models_dev_button_state: MouseStateHandle,
    remove_button_state: MouseStateHandle,
    add_model_button_state: MouseStateHandle,
    header_rows: Vec<HeaderRow>,
    add_header_button_state: MouseStateHandle,
    /// Mouse state for each of the 5 ApiType chips, keyed by chip display name.
    api_type_chip_states: RefCell<HashMap<AgentProviderApiType, MouseStateHandle>>,
    model_rows: Vec<ModelRow>,
}

/// Custom Agent Provider settings widget.
pub(super) struct AgentProvidersWidget {
    add_button_state: MouseStateHandle,
    refresh_catalog_button_state: MouseStateHandle,
    expand_chips_button_state: MouseStateHandle,
    /// Search box for the quick-add chip row.
    search_editor: ViewHandle<EditorView>,
    /// One button state per catalog provider id — used by the chip row.
    quick_add_button_states: RefCell<HashMap<String, MouseStateHandle>>,
    rows: RefCell<HashMap<String, ProviderRow>>,
}

impl AgentProvidersWidget {
    pub(super) fn new(ctx: &mut ViewContext<AISettingsPageView>) -> Self {
        let providers = AISettings::as_ref(ctx).agent_providers.value().clone();
        let mut rows = HashMap::with_capacity(providers.len());
        for provider in &providers {
            let row = Self::build_row(provider, ctx);
            rows.insert(provider.id.clone(), row);
        }

        // Trigger a catalog load on page entry (disk cache + network if needed).
        ctx.dispatch_typed_action_deferred(AISettingsPageAction::EnsureModelsDevLoaded);

        // ---- Search box ----
        let initial_query = crate::ai::agent_providers::models_dev::search_query();
        let search_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(
                crate::t!("settings-agent-providers-search-placeholder"),
                ctx,
            );
            if !initial_query.is_empty() {
                editor.set_buffer_text(&initial_query, ctx);
            }
            editor
        });
        ctx.subscribe_to_view(&search_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Edited(_)) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(AISettingsPageAction::SetModelsDevSearchQuery(
                    buffer_text,
                ));
            }
        });

        Self {
            add_button_state: MouseStateHandle::default(),
            refresh_catalog_button_state: MouseStateHandle::default(),
            expand_chips_button_state: MouseStateHandle::default(),
            search_editor,
            quick_add_button_states: RefCell::new(HashMap::new()),
            rows: RefCell::new(rows),
        }
    }

    /// Build the EditorView handles and subscriptions for a single model row.
    fn build_model_row(
        provider_id: &str,
        model_index: usize,
        model: &AgentProviderModel,
        ctx: &mut ViewContext<AISettingsPageView>,
    ) -> ModelRow {
        // ---- name editor ----
        let initial_name = model.name.clone();
        let name_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(
                crate::t!("settings-agent-providers-model-name-placeholder"),
                ctx,
            );
            if !initial_name.is_empty() {
                editor.set_buffer_text(&initial_name, ctx);
            }
            editor
        });
        let provider_id_for_name = provider_id.to_owned();
        ctx.subscribe_to_view(&name_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderModelName {
                        provider_id: provider_id_for_name.clone(),
                        model_index,
                        name: buffer_text,
                    },
                );
                collapse_selection_if_blurred(&editor, event, ctx);
            }
        });

        // ---- id editor ----
        let initial_id = model.id.clone();
        let id_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(
                crate::t!("settings-agent-providers-model-id-placeholder"),
                ctx,
            );
            if !initial_id.is_empty() {
                editor.set_buffer_text(&initial_id, ctx);
            }
            editor
        });
        let provider_id_for_id = provider_id.to_owned();
        ctx.subscribe_to_view(&id_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderModelId {
                        provider_id: provider_id_for_id.clone(),
                        model_index,
                        id: buffer_text,
                    },
                );
                collapse_selection_if_blurred(&editor, event, ctx);
            }
        });

        // ---- context_window editor (numeric, empty = 0 = unspecified) ----
        let initial_context = if model.context_window == 0 {
            String::new()
        } else {
            model.context_window.to_string()
        };
        let context_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(
                crate::t!("settings-agent-providers-model-context-placeholder"),
                ctx,
            );
            if !initial_context.is_empty() {
                editor.set_buffer_text(&initial_context, ctx);
            }
            editor
        });
        let provider_id_for_ctx = provider_id.to_owned();
        ctx.subscribe_to_view(&context_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                let value = parse_token_count(&buffer_text);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderModelContextWindow {
                        provider_id: provider_id_for_ctx.clone(),
                        model_index,
                        context_window: value,
                    },
                );
                collapse_selection_if_blurred(&editor, event, ctx);
            }
        });

        // ---- max_output_tokens editor ----
        let initial_output = if model.max_output_tokens == 0 {
            String::new()
        } else {
            model.max_output_tokens.to_string()
        };
        let output_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(
                crate::t!("settings-agent-providers-model-output-placeholder"),
                ctx,
            );
            if !initial_output.is_empty() {
                editor.set_buffer_text(&initial_output, ctx);
            }
            editor
        });
        let provider_id_for_out = provider_id.to_owned();
        ctx.subscribe_to_view(&output_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                let value = parse_token_count(&buffer_text);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderModelMaxOutput {
                        provider_id: provider_id_for_out.clone(),
                        model_index,
                        max_output_tokens: value,
                    },
                );
                collapse_selection_if_blurred(&editor, event, ctx);
            }
        });

        ModelRow {
            name_editor,
            id_editor,
            context_editor,
            output_editor,
            remove_button_state: MouseStateHandle::default(),
            expand_button_state: MouseStateHandle::default(),
            image_chip_state: MouseStateHandle::default(),
            pdf_chip_state: MouseStateHandle::default(),
            audio_chip_state: MouseStateHandle::default(),
            reasoning_chip_state: MouseStateHandle::default(),
            tool_call_chip_state: MouseStateHandle::default(),
        }
    }

    fn build_header_row(
        provider_id: &str,
        header_index: usize,
        key: &str,
        value: &str,
        ctx: &mut ViewContext<AISettingsPageView>,
    ) -> HeaderRow {
        let initial_key = key.to_owned();
        let key_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("x-portkey-provider", ctx);
            if !initial_key.is_empty() {
                editor.set_buffer_text(&initial_key, ctx);
            }
            editor
        });

        let initial_value = value.to_owned();
        let val_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("openai", ctx);
            if !initial_value.is_empty() {
                editor.set_buffer_text(&initial_value, ctx);
            }
            editor
        });

        let provider_id_for_key = provider_id.to_owned();
        let val_editor_for_key = val_editor.clone();
        ctx.subscribe_to_view(&key_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let key = editor.as_ref(ctx).buffer_text(ctx);
                let value = val_editor_for_key.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderHeader {
                        provider_id: provider_id_for_key.clone(),
                        header_index,
                        key,
                        value,
                    },
                );
                collapse_selection_if_blurred(&editor, event, ctx);
            }
        });

        let provider_id_for_value = provider_id.to_owned();
        let key_editor_for_value = key_editor.clone();
        ctx.subscribe_to_view(&val_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let key = key_editor_for_value.as_ref(ctx).buffer_text(ctx);
                let value = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderHeader {
                        provider_id: provider_id_for_value.clone(),
                        header_index,
                        key,
                        value,
                    },
                );
                collapse_selection_if_blurred(&editor, event, ctx);
            }
        });

        HeaderRow {
            key_editor,
            val_editor,
            remove_button_state: MouseStateHandle::default(),
        }
    }

    /// Build all view handles and button mouse states for a single provider.
    fn build_row(
        provider: &AgentProvider,
        ctx: &mut ViewContext<AISettingsPageView>,
    ) -> ProviderRow {
        let provider_id = provider.id.clone();

        // ---- Name editor ----
        let initial_name = provider.name.clone();
        let name_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor
                .set_placeholder_text(crate::t!("settings-agent-providers-name-placeholder"), ctx);
            if !initial_name.is_empty() {
                editor.set_buffer_text(&initial_name, ctx);
            }
            editor
        });
        let provider_id_for_name = provider_id.clone();
        ctx.subscribe_to_view(&name_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(AISettingsPageAction::UpdateAgentProviderName {
                    provider_id: provider_id_for_name.clone(),
                    name: buffer_text,
                });
                collapse_selection_if_blurred(&editor, event, ctx);
            }
        });

        // ---- Base URL editor ----
        let initial_base_url = provider.base_url.clone();
        let base_url_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(
                crate::t!("settings-agent-providers-base-url-placeholder"),
                ctx,
            );
            if !initial_base_url.is_empty() {
                editor.set_buffer_text(&initial_base_url, ctx);
            }
            editor
        });
        let provider_id_for_url = provider_id.clone();
        ctx.subscribe_to_view(&base_url_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderBaseUrl {
                        provider_id: provider_id_for_url.clone(),
                        base_url: buffer_text,
                    },
                );
                collapse_selection_if_blurred(&editor, event, ctx);
            }
        });

        // ---- API Key editor (password mode) ----
        let initial_api_key = AgentProviderSecrets::as_ref(ctx)
            .get(&provider_id)
            .map(str::to_owned)
            .unwrap_or_default();
        let api_key_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, true);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(
                crate::t!("settings-agent-providers-api-key-placeholder"),
                ctx,
            );
            if !initial_api_key.is_empty() {
                editor.set_buffer_text(&initial_api_key, ctx);
            }
            editor
        });
        let provider_id_for_key = provider_id.clone();
        ctx.subscribe_to_view(&api_key_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderApiKey {
                        provider_id: provider_id_for_key.clone(),
                        api_key: buffer_text,
                    },
                );
                collapse_selection_if_blurred(&editor, event, ctx);
            }
        });

        // ---- Model rows ----
        let model_rows: Vec<ModelRow> = provider
            .models
            .iter()
            .enumerate()
            .map(|(idx, m)| Self::build_model_row(&provider_id, idx, m, ctx))
            .collect();

        let header_rows: Vec<HeaderRow> = provider
            .extra_headers
            .iter()
            .enumerate()
            .map(|(idx, (k, v))| Self::build_header_row(&provider_id, idx, k, v, ctx))
            .collect();
        let add_header_button_state = MouseStateHandle::default();

        ProviderRow {
            name_editor,
            base_url_editor,
            api_key_editor,
            fetch_button_state: MouseStateHandle::default(),
            sync_models_dev_button_state: MouseStateHandle::default(),
            remove_button_state: MouseStateHandle::default(),
            add_model_button_state: MouseStateHandle::default(),
            header_rows,
            add_header_button_state,
            api_type_chip_states: RefCell::new(HashMap::new()),
            model_rows,
        }
    }

    /// Render the "API Type" row: 5 chips in a row, with the selected one highlighted.
    /// Clicking a chip dispatches `SetAgentProviderApiType`; the backend fills in the default endpoint.
    fn render_api_type_field(
        &self,
        provider: &AgentProvider,
        row: &ProviderRow,
        label_color: warp_core::ui::theme::Fill,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let label_text = Container::new(
            Text::new(
                crate::t!("settings-agent-providers-field-api-type"),
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(label_color.into())
            .finish(),
        )
        .with_margin_top(FIELD_LABEL_MARGIN_TOP)
        .with_margin_bottom(FIELD_LABEL_MARGIN_BOTTOM)
        .finish();

        let mut chip_row = Flex::row().with_cross_axis_alignment(CrossAxisAlignment::Center);
        {
            let mut states = row.api_type_chip_states.borrow_mut();
            for variant in AgentProviderApiType::iter() {
                let state = states
                    .entry(variant)
                    .or_insert_with(MouseStateHandle::default)
                    .clone();
                let is_selected = provider.api_type == variant;
                let label = if is_selected {
                    format!("● {}", variant.display_name())
                } else {
                    variant.display_name().to_owned()
                };
                let chip = Self::render_card_button(
                    label,
                    state,
                    AISettingsPageAction::SetAgentProviderApiType {
                        provider_id: provider.id.clone(),
                        api_type: variant,
                    },
                    appearance,
                );
                chip_row = chip_row.with_child(Container::new(chip).with_margin_right(6.).finish());
            }
        }

        let hint_text = Container::new(
            Text::new(
                crate::t!(
                    "settings-agent-providers-api-type-hint",
                    url = provider.api_type.default_base_url()
                ),
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(appearance.theme().disabled_ui_text_color().into())
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_top(2.)
        .finish();

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(label_text)
            .with_child(chip_row.finish())
            .with_child(hint_text)
            .finish()
    }

    fn render_card_button(
        label: impl Into<String>,
        mouse_state: MouseStateHandle,
        action: AISettingsPageAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, mouse_state)
            .with_style(UiComponentStyles {
                font_size: Some(CARD_BUTTON_FONT_SIZE),
                padding: Some(Coords::uniform(CARD_BUTTON_PADDING)),
                ..Default::default()
            })
            .with_centered_text_label(label.into())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(action.clone());
            })
            .finish()
    }

    fn render_model_row(
        provider: &AgentProvider,
        index: usize,
        model: &AgentProviderModel,
        row: &ModelRow,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let provider_id = provider.id.as_str();
        let is_expanded = is_model_expanded(provider_id, index);

        // chevron: expanded ▾ / collapsed ▸. Reuse render_card_button visual style.
        let chevron_label = if is_expanded { "▾" } else { "▸" };
        let chevron_button = Self::render_card_button(
            chevron_label,
            row.expand_button_state.clone(),
            AISettingsPageAction::ToggleAgentProviderModelExpanded {
                provider_id: provider.id.clone(),
                model_index: index,
            },
            appearance,
        );

        let cell = |flex: f32, view: &ViewHandle<EditorView>| -> Box<dyn Element> {
            Expanded::new(
                flex,
                Container::new(ChildView::new(view).finish())
                    .with_margin_right(MODEL_ROW_GAP)
                    .finish(),
            )
            .finish()
        };

        let header_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(cell(2., &row.name_editor))
            .with_child(cell(2., &row.id_editor))
            .with_child(cell(1., &row.context_editor))
            .with_child(cell(1., &row.output_editor))
            .with_child(chevron_button)
            .finish();

        let mut col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(header_row);

        if is_expanded {
            col = col.with_child(Self::render_model_detail_panel(
                provider, index, model, row, appearance,
            ));
        }

        Container::new(col.finish())
            .with_margin_bottom(MODEL_ROW_GAP)
            .finish()
    }

    /// Expanded detail panel for a single model:
    /// - Modalities: image / pdf / audio three-state chips (Auto / On / Off)
    /// - Capabilities: reasoning / tool_call bool chips
    /// - Remove button at the bottom
    fn render_model_detail_panel(
        provider: &AgentProvider,
        index: usize,
        model: &AgentProviderModel,
        row: &ModelRow,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let label_color = theme.active_ui_text_color();

        // ---- Modalities section ----
        let modalities_label = Container::new(
            Text::new(
                "Modalities".to_string(),
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(label_color.into())
            .finish(),
        )
        .with_margin_top(FIELD_LABEL_MARGIN_TOP)
        .with_margin_bottom(FIELD_LABEL_MARGIN_BOTTOM)
        .finish();

        let modality_chip = |label: &str,
                             slot: Option<bool>,
                             state: MouseStateHandle,
                             kind: ModelCapabilityKind|
         -> Box<dyn Element> {
            // Three-state visuals: Auto = bare label / On = `● label` / Off = `○ label`.
            // Follows the existing ApiType chip `● {label}` selected style.
            // Off uses hollow ○ to contrast with solid ●; Auto has no prefix (matches unselected).
            let chip_label = match slot {
                None => label.to_string(),
                Some(true) => format!("● {label}"),
                Some(false) => format!("○ {label}"),
            };
            Self::render_card_button(
                chip_label,
                state,
                AISettingsPageAction::CycleAgentProviderModelCapability {
                    provider_id: provider.id.clone(),
                    model_index: index,
                    kind,
                },
                appearance,
            )
        };

        let modalities_row = Wrap::row()
            .with_spacing(6.)
            .with_run_spacing(4.)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(modality_chip(
                "Image",
                model.image,
                row.image_chip_state.clone(),
                ModelCapabilityKind::Image,
            ))
            .with_child(modality_chip(
                "PDF",
                model.pdf,
                row.pdf_chip_state.clone(),
                ModelCapabilityKind::Pdf,
            ))
            .with_child(modality_chip(
                "Audio",
                model.audio,
                row.audio_chip_state.clone(),
                ModelCapabilityKind::Audio,
            ))
            .finish();

        // ---- Capabilities section (reasoning / tool_call) ----
        let capabilities_label = Container::new(
            Text::new(
                "Capabilities".to_string(),
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(label_color.into())
            .finish(),
        )
        .with_margin_top(FIELD_LABEL_MARGIN_TOP)
        .with_margin_bottom(FIELD_LABEL_MARGIN_BOTTOM)
        .finish();

        let bool_chip = |label: &str,
                         on: bool,
                         state: MouseStateHandle,
                         action: AISettingsPageAction|
         -> Box<dyn Element> {
            let chip_label = if on {
                format!("● {label}")
            } else {
                format!("○ {label}")
            };
            Self::render_card_button(chip_label, state, action, appearance)
        };

        let capabilities_row = Wrap::row()
            .with_spacing(6.)
            .with_run_spacing(4.)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(bool_chip(
                "Reasoning",
                model.reasoning,
                row.reasoning_chip_state.clone(),
                AISettingsPageAction::ToggleAgentProviderModelReasoning {
                    provider_id: provider.id.clone(),
                    model_index: index,
                },
            ))
            .with_child(bool_chip(
                "Tool Calling",
                model.tool_call,
                row.tool_call_chip_state.clone(),
                AISettingsPageAction::ToggleAgentProviderModelToolCall {
                    provider_id: provider.id.clone(),
                    model_index: index,
                },
            ))
            .finish();

        // ---- Remove button (only shown when expanded, to prevent accidental deletion) ----
        let remove_button = Self::render_card_button(
            "Remove model",
            row.remove_button_state.clone(),
            AISettingsPageAction::RemoveAgentProviderModel {
                provider_id: provider.id.clone(),
                model_index: index,
            },
            appearance,
        );

        let remove_row = Container::new(
            Flex::row()
                .with_main_axis_alignment(MainAxisAlignment::End)
                .with_child(remove_button)
                .finish(),
        )
        .with_margin_top(FIELD_LABEL_MARGIN_TOP)
        .finish();

        // Detail panel uses a slightly indented + bordered style to visually separate from the main row.
        Container::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(modalities_label)
                .with_child(modalities_row)
                .with_child(capabilities_label)
                .with_child(capabilities_row)
                .with_child(remove_row)
                .finish(),
        )
        .with_margin_top(4.)
        .with_margin_left(12.)
        .with_margin_bottom(8.)
        .finish()
    }

    fn render_provider_card(
        &self,
        provider: &AgentProvider,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        let label_color = if is_any_ai_enabled {
            appearance.theme().active_ui_text_color()
        } else {
            appearance.theme().disabled_ui_text_color()
        };
        let detail_color = if is_any_ai_enabled {
            appearance.theme().foreground()
        } else {
            appearance.theme().disabled_ui_text_color()
        };

        let rows = self.rows.borrow();
        let row = match rows.get(&provider.id) {
            Some(row) => row,
            None => {
                return Container::new(
                    Text::new(
                        crate::t!(
                            "settings-agent-providers-row-missing",
                            id = provider.id.as_str()
                        ),
                        appearance.ui_font_family(),
                        appearance.ui_font_size(),
                    )
                    .with_color(detail_color.into())
                    .finish(),
                )
                .with_margin_bottom(8.)
                .finish();
            }
        };

        let name_field = field_block(
            &crate::t!("settings-agent-providers-field-name"),
            ChildView::new(&row.name_editor).finish(),
            label_color,
            appearance,
        );
        let api_type_field = self.render_api_type_field(provider, row, label_color, appearance);
        let base_url_field = field_block(
            &crate::t!("settings-agent-providers-field-base-url"),
            ChildView::new(&row.base_url_editor).finish(),
            label_color,
            appearance,
        );
        let api_key_field = field_block(
            &crate::t!("settings-agent-providers-field-api-key"),
            ChildView::new(&row.api_key_editor).finish(),
            label_color,
            appearance,
        );

        let headers_label = Container::new(
            Text::new(
                "Extra Headers".to_string(),
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(label_color.into())
            .finish(),
        )
        .with_margin_top(FIELD_LABEL_MARGIN_TOP)
        .with_margin_bottom(FIELD_LABEL_MARGIN_BOTTOM)
        .finish();
        let mut headers_column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(headers_label);

        for (idx, h_row) in row.header_rows.iter().enumerate() {
            let remove_header_button = Self::render_card_button(
                "×",
                h_row.remove_button_state.clone(),
                AISettingsPageAction::RemoveAgentProviderHeader {
                    provider_id: provider.id.clone(),
                    header_index: idx,
                },
                appearance,
            );
            let header_row = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Expanded::new(
                        1.,
                        Container::new(ChildView::new(&h_row.key_editor).finish())
                            .with_margin_right(MODEL_ROW_GAP)
                            .finish(),
                    )
                    .finish(),
                )
                .with_child(
                    Expanded::new(
                        1.,
                        Container::new(ChildView::new(&h_row.val_editor).finish())
                            .with_margin_right(MODEL_ROW_GAP)
                            .finish(),
                    )
                    .finish(),
                )
                .with_child(remove_header_button)
                .finish();
            headers_column.add_child(
                Container::new(header_row)
                    .with_margin_bottom(MODEL_ROW_GAP)
                    .finish(),
            );
        }

        let add_header_button = Self::render_card_button(
            "+ Add Header",
            row.add_header_button_state.clone(),
            AISettingsPageAction::AddAgentProviderHeader {
                provider_id: provider.id.clone(),
            },
            appearance,
        );
        headers_column.add_child(add_header_button);

        // ---- Model list section ----
        let models_label = Container::new(
            Text::new(
                crate::t!(
                    "settings-agent-providers-models-label",
                    count = provider.models.len()
                ),
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(label_color.into())
            .finish(),
        )
        .with_margin_top(FIELD_LABEL_MARGIN_TOP)
        .with_margin_bottom(FIELD_LABEL_MARGIN_BOTTOM)
        .finish();

        let mut models_column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(models_label);

        if provider.models.is_empty() {
            let empty_hint = Container::new(
                Text::new(
                    crate::t!("settings-agent-providers-models-empty-hint"),
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_color(appearance.theme().disabled_ui_text_color().into())
                .soft_wrap(true)
                .finish(),
            )
            .with_margin_bottom(MODEL_ROW_GAP)
            .finish();
            models_column.add_child(empty_hint);
        } else {
            // Header: Display Name | Model ID | Context | Output
            let dim = appearance.theme().disabled_ui_text_color();
            let header_cell = |flex: f32, label: &str| -> Box<dyn Element> {
                Expanded::new(
                    flex,
                    Container::new(
                        Text::new(
                            label.to_string(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .with_margin_right(MODEL_ROW_GAP)
                    .finish(),
                )
                .finish()
            };
            let header = Container::new(
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(header_cell(
                        2.,
                        &crate::t!("settings-agent-providers-models-header-name"),
                    ))
                    .with_child(header_cell(
                        2.,
                        &crate::t!("settings-agent-providers-models-header-id"),
                    ))
                    .with_child(header_cell(
                        1.,
                        &crate::t!("settings-agent-providers-models-header-context"),
                    ))
                    .with_child(header_cell(
                        1.,
                        &crate::t!("settings-agent-providers-models-header-output"),
                    ))
                    // Spacer to align with the × buttons below
                    .with_child(
                        Text::new(
                            "  ".to_string(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .finish(),
            )
            .with_margin_bottom(2.)
            .finish();
            models_column.add_child(header);

            for (idx, m_row) in row.model_rows.iter().enumerate() {
                let model = match provider.models.get(idx) {
                    Some(m) => m,
                    // Edge case: settings changed during rebuild; model_rows and provider.models
                    // lengths temporarily mismatched; skip to avoid panic, next frame will self-correct.
                    None => continue,
                };
                models_column.add_child(Self::render_model_row(
                    provider, idx, model, m_row, appearance,
                ));
            }
        }

        // ---- Bottom button row ----
        let add_model_button = Self::render_card_button(
            crate::t!("settings-agent-providers-add-model"),
            row.add_model_button_state.clone(),
            AISettingsPageAction::AddAgentProviderModel {
                provider_id: provider.id.clone(),
            },
            appearance,
        );
        let fetch_button = Self::render_card_button(
            crate::t!("settings-agent-providers-fetch-from-api"),
            row.fetch_button_state.clone(),
            AISettingsPageAction::FetchAgentProviderModels {
                provider_id: provider.id.clone(),
            },
            appearance,
        );
        let sync_models_dev_button = Self::render_card_button(
            crate::t!("settings-agent-providers-sync-models-dev"),
            row.sync_models_dev_button_state.clone(),
            AISettingsPageAction::SyncProviderModelsFromModelsDev {
                provider_id: provider.id.clone(),
            },
            appearance,
        );
        let remove_button = Self::render_card_button(
            crate::t!("settings-agent-providers-remove"),
            row.remove_button_state.clone(),
            AISettingsPageAction::RemoveAgentProvider {
                provider_id: provider.id.clone(),
            },
            appearance,
        );

        let bottom_row = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(
                        Container::new(add_model_button)
                            .with_margin_right(8.)
                            .finish(),
                    )
                    .with_child(Container::new(fetch_button).with_margin_right(8.).finish())
                    .with_child(sync_models_dev_button)
                    .finish(),
            )
            .with_child(remove_button)
            .finish();

        // Dummy read of detail_color to suppress unused warning; reserved for potential coloring.
        let _ = detail_color;

        Container::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(name_field)
                .with_child(api_type_field)
                .with_child(base_url_field)
                .with_child(api_key_field)
                .with_child(
                    Container::new(headers_column.finish())
                        .with_margin_top(8.)
                        .finish(),
                )
                .with_child(
                    Container::new(models_column.finish())
                        .with_margin_top(8.)
                        .finish(),
                )
                .with_child(Container::new(bottom_row).with_margin_top(10.).finish())
                .finish(),
        )
        .with_background(appearance.theme().surface_1())
        .with_uniform_padding(12.)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
        .with_margin_bottom(8.)
        .finish()
    }
}

/// Parse user input as a token count. Accepts `128k` / `128K` / `128 000` / `128,000` / whitespace;
/// parse failures return 0 (meaning: unspecified).
fn parse_token_count(input: &str) -> u32 {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '_')
        .collect();
    if cleaned.is_empty() {
        return 0;
    }
    let lower = cleaned.to_lowercase();
    let (num_part, multiplier): (&str, u64) = if let Some(stripped) = lower.strip_suffix('k') {
        (stripped, 1_000)
    } else if let Some(stripped) = lower.strip_suffix('m') {
        (stripped, 1_000_000)
    } else {
        (lower.as_str(), 1)
    };
    num_part
        .parse::<f64>()
        .ok()
        .map(|n| (n * multiplier as f64).round() as u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}

/// Collapse the editor selection to the end when blurred.
///
/// Each input field is an independent `EditorView` maintaining its own selection range.
/// Selection highlight rendering is unaffected by focus state (see `app/src/editor/view/element.rs:1091`),
/// so after double/triple-click or drag selection, losing focus leaves the old selection on the buffer,
/// displayed alongside other editors' selections — looks like "multiple selected" state.
/// Collapsing head/tail to the end on Blur visually releases the selection.
fn collapse_selection_if_blurred(
    editor: &ViewHandle<EditorView>,
    event: &EditorEvent,
    ctx: &mut ViewContext<AISettingsPageView>,
) {
    if matches!(event, EditorEvent::Blurred) {
        editor.update(ctx, |editor, ctx| editor.move_to_buffer_end(ctx));
    }
}

fn single_line_editor_options(
    appearance: &Appearance,
    is_password: bool,
) -> SingleLineEditorOptions {
    SingleLineEditorOptions {
        is_password,
        clear_selections_on_blur: true,
        text: TextOptions {
            font_size_override: Some(appearance.ui_font_size()),
            font_family_override: Some(appearance.monospace_font_family()),
            text_colors_override: Some(TextColors {
                default_color: appearance.theme().active_ui_text_color(),
                disabled_color: appearance.theme().disabled_ui_text_color(),
                hint_color: appearance.theme().disabled_ui_text_color(),
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn field_block(
    label: &str,
    editor_element: Box<dyn Element>,
    label_color: warp_core::ui::theme::Fill,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let label_text = Container::new(
        Text::new(
            label.to_string(),
            appearance.ui_font_family(),
            appearance.ui_font_size(),
        )
        .with_color(label_color.into())
        .finish(),
    )
    .with_margin_top(FIELD_LABEL_MARGIN_TOP)
    .with_margin_bottom(FIELD_LABEL_MARGIN_BOTTOM)
    .finish();

    Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_child(label_text)
        .with_child(editor_element)
        .finish()
}

impl AgentProvidersWidget {
    /// Render the "quick add known providers from models.dev" section:
    /// - Title + "Refresh catalog" button
    /// - A chip row (one per catalog provider id); clicking creates a local provider and pre-fills models
    /// - Shows "Loading..." while the catalog is not yet loaded
    fn render_models_dev_section(
        &self,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        use crate::ai::agent_providers::models_dev;

        let label_color = appearance.theme().active_ui_text_color();
        let dim_color = appearance.theme().disabled_ui_text_color();

        let title = Text::new(
            crate::t!("settings-agent-providers-quick-add-title"),
            appearance.ui_font_family(),
            appearance.ui_font_size(),
        )
        .with_color(label_color.into())
        .finish();

        let refresh_button = Self::render_card_button(
            crate::t!("settings-agent-providers-refresh-catalog"),
            self.refresh_catalog_button_state.clone(),
            AISettingsPageAction::RefreshModelsDev,
            appearance,
        );

        let search_box = Container::new(ChildView::new(&self.search_editor).finish())
            .with_margin_left(8.)
            .with_margin_right(8.)
            .finish();

        let header_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(title)
            .with_child(Expanded::new(1., search_box).finish())
            .with_child(refresh_button)
            .finish();

        let mut body = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        body.add_child(header_row);

        // Collapsed: show first N items (enough for ~1 row; actual wrapping is handled by Wrap layout).
        const COLLAPSED_LIMIT: usize = 8;
        let expanded = models_dev::chips_expanded();

        match models_dev::cached() {
            None => {
                body.add_child(
                    Container::new(
                        Text::new(
                            crate::t!("settings-agent-providers-loading-catalog"),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(dim_color.into())
                        .finish(),
                    )
                    .with_margin_top(4.)
                    .finish(),
                );
            }
            Some(catalog) if catalog.is_empty() => {
                body.add_child(
                    Container::new(
                        Text::new(
                            crate::t!("settings-agent-providers-catalog-empty"),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(dim_color.into())
                        .finish(),
                    )
                    .with_margin_top(4.)
                    .finish(),
                );
            }
            Some(catalog) => {
                // Filter by search query; empty query → all entries in order.
                let query = models_dev::search_query();
                let filtered = models_dev::filter_catalog(&catalog, &query);
                let total = filtered.len();
                let has_query = !query.trim().is_empty();
                // When searching, always expand all matches (otherwise results ≤ collapse limit won't be fully visible).
                let visible_count = if expanded || has_query {
                    total
                } else {
                    COLLAPSED_LIMIT.min(total)
                };

                let mut wrap = Wrap::row()
                    .with_spacing(6.)
                    .with_run_spacing(6.)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center);
                {
                    let mut states = self.quick_add_button_states.borrow_mut();
                    for (cat_id, cat_provider) in filtered.iter().take(visible_count) {
                        let label = if cat_provider.name.is_empty() {
                            cat_id.clone()
                        } else {
                            cat_provider.name.clone()
                        };
                        let state = states
                            .entry(cat_id.clone())
                            .or_insert_with(MouseStateHandle::default)
                            .clone();
                        let model_count = cat_provider.models.len();
                        let display_label = format!("+ {label} ({model_count})");
                        let chip = Self::render_card_button(
                            display_label,
                            state,
                            AISettingsPageAction::AddProviderFromModelsDev {
                                catalog_provider_id: cat_id.clone(),
                            },
                            appearance,
                        );
                        wrap = wrap.with_child(chip);
                    }
                }
                body.add_child(Container::new(wrap.finish()).with_margin_top(4.).finish());

                if has_query && total == 0 {
                    body.add_child(
                        Container::new(
                            Text::new(
                                crate::t!(
                                    "settings-agent-providers-no-match",
                                    query = query.as_str()
                                ),
                                appearance.ui_font_family(),
                                appearance.ui_font_size(),
                            )
                            .with_color(dim_color.into())
                            .finish(),
                        )
                        .with_margin_top(4.)
                        .finish(),
                    );
                }

                // Expand/collapse button (only shown when not searching + catalog has more than the collapse limit).
                if !has_query && total > COLLAPSED_LIMIT {
                    let toggle_label = if expanded {
                        crate::t!("settings-agent-providers-collapse")
                    } else {
                        let count: i64 = (total - COLLAPSED_LIMIT) as i64;
                        crate::t!("settings-agent-providers-expand-remaining", count = count)
                    };
                    let toggle_button = Self::render_card_button(
                        toggle_label,
                        self.expand_chips_button_state.clone(),
                        AISettingsPageAction::ToggleModelsDevChipsExpanded,
                        appearance,
                    );
                    body.add_child(
                        Container::new(
                            Flex::row()
                                .with_main_axis_alignment(MainAxisAlignment::Start)
                                .with_child(toggle_button)
                                .finish(),
                        )
                        .with_margin_top(6.)
                        .finish(),
                    );
                }
            }
        }

        Container::new(body.finish())
            .with_background(appearance.theme().surface_1())
            .with_uniform_padding(10.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_margin_bottom(10.)
            .finish()
    }
}

impl SettingsWidget for AgentProvidersWidget {
    type View = AISettingsPageView;

    fn search_terms(&self) -> &str {
        "agent provider providers custom openai compatible deepseek glm moonshot dashscope qwen ollama base url api key models 提供商 自定义 模型"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        let providers = AISettings::as_ref(app).agent_providers.value().clone();

        let title_node = build_sub_header(
            appearance,
            crate::t!("settings-agent-providers-title"),
            Some(if is_any_ai_enabled {
                appearance.theme().active_ui_text_color()
            } else {
                appearance.theme().disabled_ui_text_color()
            }),
        )
        .finish();

        let header_add_button = Self::render_card_button(
            crate::t!("settings-agent-providers-add-button"),
            self.add_button_state.clone(),
            AISettingsPageAction::AddAgentProvider,
            appearance,
        );

        let header = Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(Expanded::new(1., title_node).finish())
                .with_child(header_add_button)
                .finish(),
        )
        .with_padding_bottom(HEADER_PADDING)
        .finish();

        let description_text = crate::t!("settings-agent-providers-description");
        let description = Container::new(
            Text::new(
                description_text,
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(if is_any_ai_enabled {
                appearance.theme().foreground().into()
            } else {
                appearance.theme().disabled_ui_text_color().into()
            })
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_bottom(12.)
        .finish();

        let mut column = Flex::column().with_child(header).with_child(description);

        // ---- Quick-add chip row from models.dev ----
        column.add_child(self.render_models_dev_section(appearance, app));

        if providers.is_empty() {
            let empty = Container::new(
                Text::new(
                    crate::t!("settings-agent-providers-empty"),
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_color(appearance.theme().disabled_ui_text_color().into())
                .finish(),
            )
            .with_margin_bottom(12.)
            .finish();
            column.add_child(empty);
        } else {
            for provider in &providers {
                column.add_child(self.render_provider_card(provider, appearance, app));
            }
        }

        Container::new(column.finish())
            .with_margin_bottom(HEADER_PADDING)
            .finish()
    }
}
