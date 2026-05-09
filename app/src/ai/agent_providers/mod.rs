//! Custom BYOE (Bring Your Own Endpoint) agent provider support.
//!
//! This module is responsible for:
//! - Storing each provider's `api_key` securely in the OS keychain (secure_storage),
//!   while provider metadata (name/base_url/model list) lives in settings.toml.
//! - Calling `${base_url}/models` via `OpenAiCompatibleClient` to fetch the list of
//!   available upstream models (used by the "Fetch from API" button in Settings).
//!
//! In a future phase this will implement the `AiProvider` trait to route
//! multi-agent calls to local providers.

pub mod active_ai;
pub mod attachment_caps;
pub mod chat_stream;
pub mod llm_id;
pub mod models_dev;
pub mod oneshot;
pub mod openai_compatible;
pub mod prompt_renderer;
pub mod reasoning;
pub mod secrets;
pub mod tools;
pub mod user_context;

// Current external usage:
// - `fetch_openai_compatible_models`: FetchAgentProviderModels handler in ai_page.rs
// - `AgentProviderSecrets`: multiple handlers in ai_page.rs and the lib.rs registration point
// Other symbols (OpenAiCompatibleError / OpenAiCompatibleModel / AgentProviderSecretsEvent)
// remain accessible via their full paths; not re-exported here to avoid unused_imports warnings.
pub use openai_compatible::fetch_openai_compatible_models;
pub use secrets::AgentProviderSecrets;

// ---------------------------------------------------------------------------
// LLMInfo synthesis: converts configured agent_providers into picker entries
// ---------------------------------------------------------------------------

use std::collections::HashMap;

use settings::Setting;
use warpui::{AppContext, SingletonEntity};

use crate::ai::llms::{
    AvailableLLMs, DisableReason, LLMContextWindow, LLMInfo, LLMProvider, LLMUsageMetadata,
    ModelsByFeature,
};
use crate::settings::{AISettings, AgentProvider};

/// Builds the list of valid (provider, model) LLMInfo pairs for the picker.
///
/// "Valid" = provider has a non-empty base_url + at least one model + an api_key in secrets.
/// Invalid providers are skipped entirely so the user can tell which ones are
/// incomplete by their absence from the picker.
fn build_byoe_llm_infos(app: &AppContext) -> Vec<LLMInfo> {
    let providers = AISettings::as_ref(app).agent_providers.value().clone();
    let secrets = AgentProviderSecrets::as_ref(app);
    let mut out = Vec::new();

    for provider in providers {
        if provider.base_url.trim().is_empty() {
            continue;
        }
        if provider.models.is_empty() {
            continue;
        }
        let has_key = secrets
            .get(&provider.id)
            .map(|k| !k.is_empty())
            .unwrap_or(false);
        if !has_key {
            continue;
        }

        let provider_label = if provider.name.trim().is_empty() {
            provider.id.clone()
        } else {
            provider.name.clone()
        };

        for model in &provider.models {
            if model.id.trim().is_empty() {
                continue;
            }
            let display_name = if model.name.trim().is_empty() {
                model.id.clone()
            } else {
                model.name.clone()
            };
            // Three-tier capability resolution: user's settings three-state chip override
            // → models.dev catalog inference → substring fallback.
            // The same function is used by chat_stream when deciding ContentPart::Binary,
            // so the UI display and runtime behavior are always consistent.
            let resolved_caps =
                attachment_caps::resolve_for_model(&provider.id, provider.api_type, model);
            let vision_supported = resolved_caps.images;
            out.push(LLMInfo {
                display_name: format!("{provider_label} / {display_name}"),
                base_model_name: format!("{provider_label} / {display_name}"),
                id: llm_id::encode(&provider.id, &model.id),
                reasoning_level: None,
                usage_metadata: LLMUsageMetadata {
                    request_multiplier: 1,
                    credit_multiplier: None,
                },
                description: None,
                disable_reason: None,
                vision_supported,
                spec: None,
                provider: LLMProvider::Unknown,
                host_configs: HashMap::new(),
                discount_percentage: None,
                context_window: LLMContextWindow::default(),
            });
        }
    }

    out
}

/// Placeholder entry used when no valid providers are configured.
/// `AvailableLLMs::new` rejects empty lists, so at least one entry is required.
/// This entry is shown as disabled (grey) and cannot be selected — it guides the
/// user to Settings → Agents → Providers to add a provider.
fn placeholder_llm_info() -> LLMInfo {
    LLMInfo {
        display_name: "No providers configured — go to Settings → Agents → Providers".to_owned(),
        base_model_name: "No provider".to_owned(),
        id: ai::LLMId::from("BYOE-placeholder"),
        reasoning_level: None,
        usage_metadata: LLMUsageMetadata {
            request_multiplier: 1,
            credit_multiplier: None,
        },
        description: None,
        disable_reason: Some(DisableReason::Unavailable),
        vision_supported: false,
        spec: None,
        provider: LLMProvider::Unknown,
        host_configs: HashMap::new(),
        discount_percentage: None,
        context_window: LLMContextWindow::default(),
    }
}

/// Builds a `ModelsByFeature` populated entirely from BYOE provider models.
/// All four features (agent_mode / coding / cli_agent / computer_use) share the same
/// model set — custom providers do not distinguish by capability.
pub fn build_BYOE_models_by_feature(app: &AppContext) -> ModelsByFeature {
    let mut choices = build_byoe_llm_infos(app);
    if choices.is_empty() {
        choices.push(placeholder_llm_info());
    }

    let default_id = choices[0].id.clone();
    let make = || {
        AvailableLLMs::new(default_id.clone(), choices.clone(), None)
            .expect("choices is non-empty by construction")
    };

    ModelsByFeature {
        agent_mode: make(),
        coding: make(),
        cli_agent: Some(make()),
        computer_use: Some(make()),
    }
}

/// Resolves a BYOE `LLMId` to `(provider, api_key, model_id)` from AISettings and secrets.
/// Returns `None` if any piece is missing; callers should map this to an `InvalidApiKey` error.
pub fn lookup_BYOE(app: &AppContext, id: &ai::LLMId) -> Option<(AgentProvider, String, String)> {
    let (provider_id, model_id) = llm_id::decode(id)?;
    let providers = AISettings::as_ref(app).agent_providers.value().clone();
    let provider = providers.into_iter().find(|p| p.id == provider_id)?;
    let api_key = AgentProviderSecrets::as_ref(app)
        .get(&provider_id)
        .map(str::to_owned)?;
    Some((provider, api_key, model_id))
}
