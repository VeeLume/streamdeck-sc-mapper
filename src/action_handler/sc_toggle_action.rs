use std::sync::Arc;

use serde::{ Deserialize, Serialize };
use serde_json::Value;
use crate::{
    action_handler::{
        send_to_property_inspector,
        show_alert,
        ActionHandler,
        KeyCoordinates,
        string_to_string_opt,
    },
    data_source::DataSourcePayload,
    logger::ActionLog,
    plugin::WriteSink,
    state::GameInstallType,
    utils::get_locked_app_state,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Settings {
    #[serde(default, deserialize_with = "string_to_string_opt")]
    action: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { action: None }
    }
}

impl Settings {
    pub fn from_json(map: &serde_json::Map<String, Value>) -> Result<Self, String> {
        let json = serde_json
            ::to_value(map)
            .map_err(|e| format!("Failed to convert settings to JSON: {}", e))?;
        serde_json::from_value(json).map_err(|e| format!("Failed to deserialize settings: {}", e))
    }

    pub fn to_json(&self) -> serde_json::Map<String, Value> {
        match serde_json::to_value(self) {
            Ok(Value::Object(obj)) => obj,
            _ => serde_json::Map::new(),
        }
    }
}

pub struct ActionToggleKey {
    logger: Arc<dyn ActionLog>,
}

impl ActionToggleKey {
    pub const ACTION_NAME: &'static str = "icu.veelume.sc-mapper.toggleaction";

    pub fn new(logger: Arc<dyn ActionLog>) -> Self {
        Self { logger }
    }
}

impl ActionHandler for ActionToggleKey {
    fn on_key_down(
        &self,
        write: WriteSink,
        context: &str,
        _device: &str,
        is_in_multi_action: bool,
        _coordinates: Option<&KeyCoordinates>,
        settings: &serde_json::Map<std::string::String, Value>,
        state: Option<u8>,
        user_desired_state: Option<u8>
    ) {
        let settings = match Settings::from_json(settings) {
            Ok(settings) => settings,
            Err(e) => {
                self.logger.log(&format!("❌ Invalid settings: {}", e));
                return;
            }
        };
        let state = match is_in_multi_action {
            true =>
                match user_desired_state {
                    Some(state) => state,
                    None => {
                        self.logger.log("❌ No state provided for multi-action toggle");
                        return;
                    }
                }
            false =>
                match state {
                    Some(state) => state,
                    None => {
                        self.logger.log("❌ No state provided for single action toggle");
                        return;
                    }
                }
        };

        if let Some(action) = settings.action {
            self.logger.log(&format!("🔄 Toggling action: {} with State {}", action, state));
            let app_state = match get_locked_app_state() {
                Ok(state) => state,
                Err(e) => {
                    self.logger.log(&format!("❌ AppState error: {}", e));
                    return;
                }
            };

            if let Err(e) = app_state.send_key(&action, None, Some(state == 0)) {
                self.logger.log(&format!("❌ Failed to send key: {}", e));
                show_alert(write, context);
            }
        }
    }

    fn on_did_receive_property_inspector_message(
        &self,
        write: WriteSink,
        context: &str,
        payload: &Value
    ) {
        let event = payload.get("event").and_then(Value::as_str);

        match event {
            Some("getActions") => {
                let mut state = match get_locked_app_state() {
                    Ok(state) => state,
                    Err(e) => {
                        self.logger.log(&format!("❌ AppState error: {}", e));
                        show_alert(write, context);
                        return;
                    }
                };

                let items = match state.get_actions(GameInstallType::Live) {
                    Ok(actions) => actions,
                    Err(()) => {
                        self.logger.log(&format!("❌ Failed to get actions"));
                        show_alert(write, context);
                        return;
                    }
                };

                send_to_property_inspector(write, context, DataSourcePayload {
                    event: Some("getActions".to_string()),
                    items: items,
                });
            }
            _ => {
                self.logger.log(
                    &format!(
                        "ℹ️ Unhandled property inspector event: {}",
                        event.unwrap_or("unknown")
                    )
                );
            }
        }
    }
}
