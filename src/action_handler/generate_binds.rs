use std::sync::{ atomic::{ AtomicBool, Ordering }, Arc, Mutex };

use serde_json::Value;
use timer::{ Guard, Timer };

use crate::{
    action_handler::{ show_alert, show_ok, ActionHandler, KeyCoordinates },
    logger::ActionLog,
    plugin::{ WriteSink, APP_STATE },
    state::GameInstallType, utils::get_locked_app_state,
};

pub struct GenerateBindsKey {
    logger: Arc<dyn ActionLog>,
    fired: Arc<AtomicBool>,
    timer: Timer,
    long_press_guard: Mutex<Option<Guard>>,
}

impl GenerateBindsKey {
    pub const ACTION_NAME: &'static str = "icu.veelume.sc-mapper.generatebinds";

    pub fn new(logger: Arc<dyn ActionLog>) -> Self {
        Self {
            logger,
            fired: Arc::new(AtomicBool::new(false)),
            timer: Timer::new(),
            long_press_guard: Mutex::new(None),
        }
    }
}

impl ActionHandler for GenerateBindsKey {
    fn on_key_down(
        &self,
        write: WriteSink,
        context: &str,
        _device: &str,
        _is_multi: bool,
        _coords: Option<&KeyCoordinates>,
        _settings: &serde_json::Map<String, Value>,
        _state: Option<u8>,
        _user_desired_state: Option<u8>
    ) {
        self.fired.store(false, Ordering::SeqCst);
        let fired = Arc::clone(&self.fired);
        let context = context.to_string();
        let logger = Arc::clone(&self.logger);

        let guard = self.timer.schedule_with_delay(chrono::Duration::milliseconds(500), move || {
            if fired.load(Ordering::SeqCst) {
                return;
            }

            fired.store(true, Ordering::SeqCst);
            logger.log("👉 Long press detected, generating binds with default");

            let writer = Arc::clone(&write);
            let mut state = match get_locked_app_state() {
                Ok(s) => s,
                Err(e) => {
                    logger.log(&format!("❌ Appstate error: {}", e));
                    show_alert(writer, &context);
                    return;
                }
            };

            if !state.parse_action_bindings(GameInstallType::Live, false) {
                logger.log("❌ Failed to generate binds");
                show_alert(writer, &context);
                return;
            }

            if
                let Err(e) = state.create_profile_xml(
                    GameInstallType::Live,
                    "SC Mapper with Default Binds"
                )
            {
                logger.log(&format!("❌ Failed to create profile XML: {e}"));
                show_alert(writer, &context);
                return;
            }

            logger.log("✅ Long press generation complete");
            show_ok(writer, &context);
        });

        if let Ok(mut task_guard) = self.long_press_guard.lock() {
            *task_guard = Some(guard);
        }
    }

    fn on_key_up(
        &self,
        write: WriteSink,
        context: &str,
        _device: &str,
        _is_multi: bool,
        _coords: Option<&KeyCoordinates>,
        _settings: &serde_json::Map<String, Value>,
        _state: Option<u8>
    ) {
        // Cancel the pending long press if it's not fired
        if let Ok(mut task_guard) = self.long_press_guard.lock() {
            *task_guard = None; // dropping the Guard cancels the task
        }

        if self.fired.load(Ordering::SeqCst) {
            self.logger.log("👋 Long press finished, skipping short press logic");
            return;
        }

        self.logger.log("👋 Short press detected, generating binds with custom");

        let mut state = match get_locked_app_state() {
            Ok(s) => s,
            Err(e) => {
                self.logger.log(&format!("❌ Appstate error: {}", e));
                show_alert(write, context);
                return;
            }

        };

        if !state.parse_action_bindings(GameInstallType::Live, true) {
            self.logger.log("❌ Failed to generate binds");
            show_alert(write, context);
            return;
        }

        if
            let Err(e) = state.create_profile_xml(
                GameInstallType::Live,
                "SC Mapper with Custom Binds"
            )
        {
            self.logger.log(&format!("{e}"));
            show_alert(write, context);
            return;
        }

        self.logger.log("✅ Short press generation complete");
        show_ok(write, context);
    }
}
