use std::collections::HashMap;
use std::sync::{ Arc, Mutex };

use once_cell::sync::OnceCell;
use websocket::{ ClientBuilder, OwnedMessage };

use crate::action_handler::sc_action::ActionKey;
use crate::{
    action_handler::{ generate_binds::GenerateBindsKey, ActionHandler },
    logger::ActionLog,
    state::AppState,
};

pub const PLUGIN_UUID: &str = "icu.veelume.sc-mapper";
pub static APP_STATE: OnceCell<Arc<Mutex<AppState>>> = OnceCell::new();

pub type WriteSink = Arc<Mutex<websocket::client::sync::Writer<std::net::TcpStream>>>;

pub enum PluginRunError {
    WebSocketError(String),
    RegistrationError(String),
    AppStateError(String),
}

pub fn run_plugin(
    url: String,
    plugin_uuid: &String,
    register_event: &String,
    logger: Arc<dyn ActionLog>
) -> Result<(), PluginRunError> {
    logger.log("🛠️ Initializing AppState...");

    let app_state = match AppState::new(Arc::clone(&logger)) {
        Ok(mut state) => {
            state.initialize();
            Arc::new(Mutex::new(state))
        }
        Err(e) => {
            logger.log(&format!("❌ Failed to initialize AppState: {e}"));
            return Err(PluginRunError::AppStateError(e.to_string()));
        }
    };

    if APP_STATE.set(app_state).is_err() {
        logger.log("❌ Failed to set AppState");
        return Err(PluginRunError::AppStateError("Failed to set AppState".to_string()));
    }

    logger.log("✅ AppState initialized");

    let client = ClientBuilder::new(&url)
        .map_err(|e| PluginRunError::WebSocketError(format!("Invalid URL: {e}")))?
        .connect_insecure()
        .map_err(|e| PluginRunError::WebSocketError(format!("WebSocket connect error: {e}")))?;

    let (mut receiver, sender) = client
        .split()
        .map_err(|e| PluginRunError::WebSocketError(format!("Failed to split WebSocket: {e}")))?;
    let write = Arc::new(Mutex::new(sender));

    let register_msg =
        serde_json::json!({
        "event": register_event,
        "uuid": plugin_uuid,
    });

    logger.log(&format!("📨 Registering plugin with UUID: {}", plugin_uuid));
    {
        let mut writer = write.lock().map_err(|e| {
            logger.log(&format!("❌ Failed to lock WebSocket writer: {}", e));
            PluginRunError::WebSocketError(format!("Failed to lock WebSocket writer: {}", e))
        })?;
        writer.send_message(&OwnedMessage::Text(register_msg.to_string())).map_err(|e| {
            logger.log(&format!("❌ Failed to send registration message: {}", e));
            PluginRunError::RegistrationError(format!("Failed to register: {e}"))
        })?;
    }

    logger.log("📨 Sent registration event to Stream Deck");

    let action_handlers: HashMap<String, Arc<dyn ActionHandler>> = HashMap::from([
        (
            GenerateBindsKey::ACTION_NAME.to_string(),
            Arc::new(GenerateBindsKey::new(Arc::clone(&logger))) as Arc<dyn ActionHandler>,
        ),
        (
            ActionKey::ACTION_NAME.to_string(),
            Arc::new(ActionKey::new(Arc::clone(&logger))) as Arc<dyn ActionHandler>,
        ),
        // Add more handlers here
    ]);

    logger.log("🔄 Starting message loop");

    for message in receiver.incoming_messages() {
        match message {
            Ok(OwnedMessage::Text(text)) => {
                logger.log(&format!("📥 Received message: {}", text));

                let msg: HashMap<String, serde_json::Value> = match serde_json::from_str(&text) {
                    Ok(val) => val,
                    Err(e) => {
                        logger.log(&format!("❌ Failed to parse message: {e}"));
                        continue;
                    }
                };

                let action = msg.get("action").and_then(|v| v.as_str());
                let event = msg.get("event").and_then(|v| v.as_str());

                if let Some(action_name) = action {
                    if let Some(handler) = action_handlers.get(action_name) {
                        handler.on_message(Arc::clone(&write), &msg);
                        logger.log(&format!("🔧 Handled action: {}", action_name));
                    } else {
                        logger.log(&format!("❗ Unknown action: {}", action_name));
                    }
                } else if let Some(evt) = event {
                    logger.log(&format!("🌀 Global event received: {evt}"));
                    // Handle global events here
                }
            }
            Ok(OwnedMessage::Close(Some(frame))) => {
                logger.log(&format!("🔌 Connection closed: {:?}", frame));
                break;
            }
            Ok(OwnedMessage::Close(None)) => {
                logger.log("🔌 Connection closed");
                break;
            }
            Ok(_) => {} // Ping, Pong, Binary, etc.
            Err(e) => {
                logger.log(&format!("❌ WebSocket error: {e}"));
                break;
            }
        }
    }

    logger.log("🛑 WebSocket loop terminated");
    Ok(())
}
