use std::sync::{ atomic::{ AtomicBool, Ordering }, Arc, Mutex };
use chrono::Duration;
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use timer::{ Guard, Timer };
use crate::{
    action_handler::{ send_to_property_inspector, show_alert, ActionHandler, KeyCoordinates, string_or_integer_to_i64, string_or_integer_to_u64_opt, string_to_string_opt },
    data_source::DataSourcePayload,
    logger::ActionLog,
    plugin::WriteSink,
    state::GameInstallType,
    utils::get_locked_app_state,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    // The long press period in milliseconds, default is 200ms, the json value is a string
    #[serde(default, deserialize_with = "string_or_integer_to_i64")]
    long_press_period: i64,
    #[serde(default, deserialize_with = "string_to_string_opt")]
    action_short: Option<String>,
    #[serde(default, deserialize_with = "string_or_integer_to_u64_opt")]
    action_short_hold: Option<u64>,
    #[serde(default, deserialize_with = "string_to_string_opt")]
    action_long: Option<String>,
    #[serde(default, deserialize_with = "string_or_integer_to_u64_opt")]
    action_long_hold: Option<u64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            action_short: None,
            action_short_hold: None,
            action_long: None,
            action_long_hold: None,
            long_press_period: 200,
        }
    }
}

impl Settings {
    pub fn from_json(map: &serde_json::Map<String, Value>) -> Result<Self, String> {
        let json = serde_json::to_value(map)
            .map_err(|e| format!("Failed to convert settings to JSON: {}", e))?;
        serde_json::from_value(json)
            .map_err(|e| format!("Failed to deserialize settings: {}", e))
    }

    pub fn to_json(&self) -> serde_json::Map<String, Value> {
        match serde_json::to_value(self) {
            Ok(Value::Object(obj)) => obj,
            _ => serde_json::Map::new(),
        }
    }
}

pub struct ActionKey {
    logger: Arc<dyn ActionLog>,
    long_fired: Arc<AtomicBool>,
    timer: Timer,
    long_press_guard: Arc<Mutex<Option<Guard>>>,
}

impl ActionKey {
    pub const ACTION_NAME: &'static str = "icu.veelume.sc-mapper.action";

    pub fn new(logger: Arc<dyn ActionLog>) -> Self {
        Self {
            logger,
            long_fired: Arc::new(AtomicBool::new(false)),
            timer: Timer::new(),
            long_press_guard: Arc::new(Mutex::new(None)),
        }
    }
}

impl ActionHandler for ActionKey {
    fn on_key_down(
        &self,
        write: WriteSink,
        context: &str,
        _device: &str,
        _is_multi: bool,
        _coords: Option<&KeyCoordinates>,
        settings: &serde_json::Map<String, serde_json::Value>,
        _state: Option<u8>,
        _user_desired_state: Option<u8>
    ) {
        self.long_fired.store(false, Ordering::SeqCst);

        let settings = match Settings::from_json(settings) {
            Ok(s) => s,
            Err(e) => {
                self.logger.log(&format!("❌ Invalid settings: {}", e));
                return;
            }
        };

        if settings.action_long.is_some() {
            let logger = Arc::clone(&self.logger);
            let action_id = match settings.action_long {
                Some(action) => action,
                None => {
                    logger.log("❌ Long press action not configured");
                    return;
                }
            };
            let hold_duration_override = settings.action_long_hold.map(|hold| std::time::Duration::from_millis(hold));
            let context = context.to_string();
            let long_fired = Arc::clone(&self.long_fired);

            let guard = self.timer.schedule_with_delay(
                Duration::milliseconds(settings.long_press_period),
                move || {
                    long_fired.store(true, Ordering::SeqCst);
                    logger.log("👉 Long press detected, executing long action");
                    let write = Arc::clone(&write);

                    let state = match get_locked_app_state() {
                        Ok(state) => state,
                        Err(e) => {
                            logger.log(&format!("❌ AppState error: {}", e));
                            show_alert(write, &context);
                            return;
                        }
                    };

                    if let Err(e) = state.send_key(&action_id, hold_duration_override, None) {
                        logger.log(&format!("❌ Failed to send long press key: {e}"));
                        show_alert(write, &context);
                    }
                }
            );

            if let Ok(mut task_guard) = self.long_press_guard.lock() {
                *task_guard = Some(guard);
            }
        }
    }

    fn on_key_up(
        &self,
        write: WriteSink,
        context: &str,
        _device: &str,
        _is_multi: bool,
        _coords: Option<&KeyCoordinates>,
        settings: &serde_json::Map<String, serde_json::Value>,
        _state: Option<u8>
    ) {
        // Cancel the pending long press if it's not fired
        if let Ok(mut task_guard) = self.long_press_guard.lock() {
            *task_guard = None; // dropping the Guard cancels the task
        }

        if self.long_fired.load(Ordering::SeqCst) {
            self.logger.log("👋 Long press ended, no action taken");
            return;
        }

        let settings = match Settings::from_json(settings) {
            Ok(s) => s,
            Err(e) => {
                self.logger.log(&format!("❌ Invalid settings: {}", e));
                return;
            }
        };

        if let Some(action) = settings.action_short {
            self.logger.log("👋 Short press detected, executing short action");

            let state = match get_locked_app_state() {
                Ok(state) => state,
                Err(e) => {
                    self.logger.log(&format!("❌ AppState error: {}", e));
                    show_alert(write, context);
                    return;
                }
            };
            let hold_duration_override = settings.action_short_hold.map(|hold| std::time::Duration::from_millis(hold));

            if let Err(e) = state.send_key(&action, hold_duration_override, None) {
                self.logger.log(&format!("❌ Failed to send short press key: {e}"));
                show_alert(write, context);
            } else {
            }
        } else {
            self.logger.log("ℹ️ No action configured for short press");
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
                    items: items
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
