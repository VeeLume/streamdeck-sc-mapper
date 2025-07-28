use std::{ collections::HashMap, sync::Arc, thread, time::Duration };
use once_cell::sync::Lazy;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

use crate::{
    action_binds::{ action_binding::ActionBinding, activation_mode::MultiTap, bind::Bind },
    logger::ActionLog,
};

impl ActionBinding {
    pub fn simulate(&self, logger: Arc<dyn ActionLog>, hold_duration_override: Option<Duration>) -> Result<(), String> {
        let bind = {
            let custom = self.custom_binds.as_ref().and_then(|b| b.keyboard.first().cloned());
            match custom.or_else(|| { self.default_binds.keyboard.first().cloned() }) {
                Some(b) => b,
                None => {
                    return Err("No keyboard bind found".to_string());
                }
            }
        };

        let mode = {
            match bind.activation_mode {
                Some(ref mode) => mode,
                None =>
                    match self.activation_mode.as_ref() {
                        Some(mode) => mode,
                        None => {
                            return Err("No activation mode defined".to_string());
                        }
                    }
            }
        };

        logger.log(&format!("🔑 Simulating key action: {}", bind.mainkey));
        logger.log(&format!("🔑 Modifiers: {:?}", bind.modifiers));
        logger.log(&format!("🔑 Activation mode: {:?}", mode));

        if mode.multi_tap == MultiTap::Two {
            logger.log("ℹ️ Sending multi-tap key action");
            for _ in 0..2 {
                if let Err(e) = send_input_combo(&bind, None) {
                    return Err(format!("Failed to send multi-tap key: {}", e));
                }
                thread::sleep(Duration::from_millis(25));
            }
            return Ok(());
        }

        if mode.on_hold || mode.press_trigger_threshold > Some(0.0) {
            let mut duration = hold_duration_override.unwrap_or_else(|| {
                if let Some(threshold) = mode.press_trigger_threshold {
                    if threshold > 0.0 { Duration::from_millis((threshold * 1000.0) as u64) } else { Duration::from_millis(260) }
                } else if let Some(delay) = mode.hold_trigger_delay {
                    if delay > 0.0 { Duration::from_millis((delay * 1000.0) as u64) } else { Duration::from_millis(260) }
                } else {
                    Duration::from_millis(260)
                }
            });
            duration = duration + Duration::from_millis(50); // Add a small buffer to ensure the hold is registered
            logger.log(&format!("⏳ Holding key for {} ms", duration.as_millis()));
            return send_input_combo(&bind, Some(duration));
        }

        if mode.on_release && !mode.on_hold && !mode.on_press {
            logger.log("ℹ️ Sending key release only, no hold or press action defined");
            return send_input_combo(&bind, None);
        }

        logger.log("ℹ️ Sending key press action");
        return send_input_combo(&bind, None);
    }
}

fn send_input_combo(bind: &Bind, hold: Option<Duration>) -> Result<(), String> {
    let mut down_events: Vec<INPUT> = Vec::new();
    let mut up_events: Vec<INPUT> = Vec::new();

    for mod_key in &bind.modifiers {
        if let Some(scan) = get_scan_code(mod_key) {
            down_events.push(build_input(scan, true, is_extended_key(mod_key)));
            up_events.push(build_input(scan, false, is_extended_key(mod_key)));
        } else {
            return Err(format!("Unknown modifier key: {}", mod_key));
        }
    }

    if let Some(scan) = get_scan_code(&bind.mainkey) {
        let main_down: INPUT = build_input(scan, true, is_extended_key(&bind.mainkey));
        let main_up: INPUT = build_input(scan, false, is_extended_key(&bind.mainkey));

        if let Some(dur) = hold {
            send_events([down_events.as_slice(), &[main_down]].concat().as_slice());
            thread::sleep(dur);
            send_events([up_events.as_slice(), &[main_up]].concat().as_slice());
        } else {
            send_events([down_events.as_slice(), &[main_down, main_up], up_events.as_slice()].concat().as_slice());
        }
    } else {
        return Err(format!("Unknown main key: {}", bind.mainkey));
    }

    return Ok(());
}

fn build_input(scan: u16, down: bool, extended: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                wScan: scan,
                dwFlags: (if down {
                    KEYEVENTF_SCANCODE
                } else {
                    KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP
                }) |
                (if extended {
                    KEYEVENTF_EXTENDEDKEY
                } else {
                    windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0)
                }),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_events(events: &[INPUT]) {
    unsafe {
        SendInput(events, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Returns the scan code for a key string (e.g., "a", "f1", "np_divide")
pub fn get_scan_code(key: &str) -> Option<u16> {
    SCAN_CODE_MAP.get(&key.to_lowercase().as_str()).copied()
}

/// Determines if the key should use the `KEYEVENTF_EXTENDEDKEY` flag.
pub fn is_extended_key(key: &str) -> bool {
    EXTENDED_KEYS.contains(&key.to_lowercase().as_str())
}

/// Static map of key names to scan codes (matches Windows SetScanCode convention)
static SCAN_CODE_MAP: Lazy<HashMap<&'static str, u16>> = Lazy::new(|| {
    HashMap::from([
        // Letters
        ("a", 0x1e),
        ("b", 0x30),
        ("c", 0x2e),
        ("d", 0x20),
        ("e", 0x12),
        ("f", 0x21),
        ("g", 0x22),
        ("h", 0x23),
        ("i", 0x17),
        ("j", 0x24),
        ("k", 0x25),
        ("l", 0x26),
        ("m", 0x32),
        ("n", 0x31),
        ("o", 0x18),
        ("p", 0x19),
        ("q", 0x10),
        ("r", 0x13),
        ("s", 0x1f),
        ("t", 0x14),
        ("u", 0x16),
        ("v", 0x2f),
        ("w", 0x11),
        ("x", 0x2d),
        ("y", 0x15),
        ("z", 0x2c),

        // Number row
        ("1", 0x02),
        ("2", 0x03),
        ("3", 0x04),
        ("4", 0x05),
        ("5", 0x06),
        ("6", 0x07),
        ("7", 0x08),
        ("8", 0x09),
        ("9", 0x0a),
        ("0", 0x0b),

        // Function keys
        ("f1", 0x3b),
        ("f2", 0x3c),
        ("f3", 0x3d),
        ("f4", 0x3e),
        ("f5", 0x3f),
        ("f6", 0x40),
        ("f7", 0x41),
        ("f8", 0x42),
        ("f9", 0x43),
        ("f10", 0x44),
        ("f11", 0x57),
        ("f12", 0x58),
        ("f13", 0x64),
        ("f14", 0x65),
        ("f15", 0x66),
        ("f16", 0x67),
        ("f17", 0x68),
        ("f18", 0x69),
        ("f19", 0x6a),
        ("f20", 0x6b),
        ("f21", 0x6c),
        ("f22", 0x6d),
        ("f23", 0x6e),
        ("f24", 0x76),

        // Modifiers
        ("lshift", 0x2a),
        ("rshift", 0x36),
        ("lctrl", 0x1d),
        ("rctrl", 0x1d),
        ("lalt", 0x38),
        ("ralt", 0x38),

        // Misc
        ("space", 0x39),
        ("tab", 0x0f),
        ("enter", 0x1c),
        ("escape", 0x01),
        ("backspace", 0x0e),
        ("[", 0x1a),
        ("lbracket", 0x1A),
        ("]", 0x1b),
        ("rbracket", 0x1B),
        ("comma", 0x33),
        ("semicolon", 0x27),
        ("apostrophe", 0x28),
        ("period", 0x34),
        ("slash", 0x35),
        ("backslash", 0x2B),
        ("minus", 0x0C),
        ("equal", 0x0D),

        // Arrow + nav keys
        ("up", 0x48),
        ("down", 0x50),
        ("left", 0x4b),
        ("right", 0x4d),
        ("pgup", 0x49),
        ("pgdn", 0x51),
        ("home", 0x47),
        ("end", 0x4f),
        ("insert", 0x52),
        ("delete", 0x53),

        // Numpad
        ("np_0", 0x52),
        ("np_1", 0x4f),
        ("np_2", 0x50),
        ("np_3", 0x51),
        ("np_4", 0x4b),
        ("np_5", 0x4c),
        ("np_6", 0x4d),
        ("np_7", 0x47),
        ("np_8", 0x48),
        ("np_9", 0x49),
        ("np_add", 0x4e),
        ("np_subtract", 0x4a),
        ("np_multiply", 0x37),
        ("np_divide", 0x35),
        ("np_period", 0x53),
    ])
});

/// Keys that require the `KEYEVENTF_EXTENDEDKEY` flag in Windows
static EXTENDED_KEYS: &[&str] = &[
    "rctrl",
    "ralt",
    "insert",
    "delete",
    "home",
    "end",
    "pgup",
    "pgdn",
    "right",
    "left",
    "down",
    "up",
    "np_divide",
];
