//! Cloud agent config — STUBBED (Phase 3 cloud removal).
//!
//! Type definitions kept for compilation; all cloud functionality is dead.
//! The module formerly held saved agent configurations synced via Warp Drive.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    cloud_object::{
        model::{
            generic_string_model::{GenericStringModel, GenericStringObjectId, StringModel},
            json_model::{JsonModel, JsonSerializer},
            persistence::CloudModel,
        },
        GenericCloudObject, GenericStringObjectFormat, GenericStringObjectUniqueKey,
        JsonObjectType, Revision, ServerCloudObject,
    },
    server::{ids::SyncId, server_api::ai::AgentConfigSnapshot, sync_queue::QueueItem},
};
use warpui::{AppContext, SingletonEntity as _};

/// Agent configuration — kept as a type shell for compile compat.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, serde_json::Value>>,
}

pub type CloudAgentConfig = GenericCloudObject<GenericStringObjectId, CloudAgentConfigModel>;
pub type CloudAgentConfigModel = GenericStringModel<AgentConfig, JsonSerializer>;

impl AgentConfig {
    pub fn to_ambient_config(&self) -> AgentConfigSnapshot {
        AgentConfigSnapshot {
            name: Some(self.name.clone()),
            environment_id: None,
            model_id: self.base_model_id.clone(),
            base_prompt: self.base_prompt.clone(),
            mcp_servers: self.mcp_servers.clone().map(|m| m.into_iter().collect()),
            profile_id: None,
            worker_host: None,
            skill_spec: None,
            computer_use_enabled: None,
            harness: None,
            harness_auth_secrets: None,
        }
    }
}

impl CloudAgentConfig {
    pub fn get_all(_app: &AppContext) -> Vec<CloudAgentConfig> {
        vec![] // Cloud sync disabled — always empty
    }

    pub fn get_by_id<'a>(_sync_id: &'a SyncId, _app: &'a AppContext) -> Option<&'a CloudAgentConfig> {
        None // Cloud sync disabled
    }
}

impl StringModel for AgentConfig {
    type CloudObjectType = CloudAgentConfig;

    fn model_type_name(&self) -> &'static str {
        "Cloud agent config"
    }

    fn should_enforce_revisions() -> bool {
        false // Cloud sync disabled
    }

    fn model_format() -> GenericStringObjectFormat {
        GenericStringObjectFormat::Json(JsonObjectType::CloudAgentConfig)
    }

    fn display_name(&self) -> String {
        self.name.clone()
    }

    fn update_object_queue_item(
        &self,
        revision_ts: Option<Revision>,
        object: &CloudAgentConfig,
    ) -> QueueItem {
        QueueItem::UpdateCloudAgentConfig {
            model: object.model().clone().into(),
            id: object.id,
            revision: revision_ts.or_else(|| object.metadata.revision.clone()),
        }
    }

    fn uniqueness_key(&self) -> Option<GenericStringObjectUniqueKey> {
        None
    }

    fn new_from_server_update(&self, _server_cloud_object: &ServerCloudObject) -> Option<Self> {
        None // Cloud sync disabled
    }

    fn should_show_activity_toasts() -> bool {
        false
    }

    fn warn_if_unsaved_at_quit() -> bool {
        false // Cloud sync disabled
    }
}

impl JsonModel for AgentConfig {
    fn json_object_type() -> JsonObjectType {
        JsonObjectType::CloudAgentConfig
    }
}
