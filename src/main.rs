use std::process::ExitCode;
use std::{ env };
use std::sync::Arc;

use crate::logger::{ ActionLog, FileLogger };
use crate::plugin::{ run_plugin, PluginRunError };
mod action_binds;
mod logger;
mod plugin;
mod state;
mod data_source;
mod action_handler;
mod keyboard_input;
mod utils;

fn main() -> ExitCode {
    let logger = match FileLogger::from_appdata() {
        Ok(logger) => Arc::new(logger),
        Err(e) => {
            eprintln!("Failed to initialize logger: {e}");
            return ExitCode::from(1);
        }
    };

    if let Err(e) = safe_main(logger.clone()) {
        let _ = logger.log(&format!("Error: {:?}", e));
        match e {
            SafeMainError::MissingPort() => {
                let _ = logger.log("Error: Missing -port argument");
                return ExitCode::from(2);
            }
            SafeMainError::MissingPluginUUID() => {
                let _ = logger.log("Error: Missing -pluginUUID argument");
                return ExitCode::from(3);
            }
            SafeMainError::MissingRegisterEvent() => {
                let _ = logger.log("Error: Missing -registerEvent argument");
                return ExitCode::from(4);
            }
            SafeMainError::PluginError(msg) => {
                let _ = logger.log(&format!("Plugin error: {}", msg));
                return ExitCode::from(5);
            }
        }
    }

    ExitCode::SUCCESS
}

#[derive(Debug)]
enum SafeMainError {
    MissingPort(),
    MissingPluginUUID(),
    MissingRegisterEvent(),
    PluginError(String),
}

fn safe_main(logger: Arc<dyn ActionLog>) -> Result<(), SafeMainError> {
    let args: Vec<String> = env::args().collect();
    let port = args
        .iter()
        .position(|a| a == "-port")
        .and_then(|i| args.get(i + 1))
        .ok_or(SafeMainError::MissingPort())?;

    let plugin_uuid = args
        .iter()
        .position(|a| a == "-pluginUUID")
        .and_then(|i| args.get(i + 1))
        .ok_or(SafeMainError::MissingPluginUUID())?;

    let register_event = args
        .iter()
        .position(|a| a == "-registerEvent")
        .and_then(|i| args.get(i + 1))
        .ok_or(SafeMainError::MissingRegisterEvent())?;

    let url = format!("ws://127.0.0.1:{port}");

    let _ = logger.log(&format!("🔌 Connecting to {url}"));

    // Delegate the actual plugin logic to another module
    run_plugin(url, plugin_uuid, register_event, logger).map_err(|e| {
        match e {
            PluginRunError::WebSocketError(msg) => SafeMainError::PluginError(msg),
            PluginRunError::RegistrationError(msg) => SafeMainError::PluginError(msg),
            PluginRunError::AppStateError(msg) => SafeMainError::PluginError(msg),
        }
    })?;

    Ok(())
}
