use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use streamdeck_lib::input::{Key, MouseButton};

use crate::bindings::constants::CANDIDATE_MODIFIERS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BindOrigin {
    #[default]
    User, // defaults + user-provided rebinds
    Generated, // produced by BindGenerator
}

// What the "main" part of a bind is
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindMain {
    Key(Key),
    Mouse(MouseButton),
    MouseWheelUp,
    MouseWheelDown,
    MouseAxis(String), // e.g. "maxis_x"
    HMD(String),       // e.g. "hmd_pitch"
    Unsupported,
}

impl fmt::Display for BindMain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BindMain::Key(k) => write!(f, "{k}"),
            BindMain::Mouse(btn) => write!(f, "{}", mouse_to_str(*btn)),
            BindMain::MouseWheelUp => write!(f, "mwheel_up"),
            BindMain::MouseWheelDown => write!(f, "mwheel_down"),
            BindMain::MouseAxis(s) => write!(f, "maxis({s})"),
            BindMain::HMD(s) => write!(f, "hmd({s})"),
            BindMain::Unsupported => write!(f, "<unsupported>"),
        }
    }
}

impl BindMain {
    pub fn is_unsupported(&self) -> bool {
        // Currently Unsupported, MouseWheelUp, MouseWheelDown, MouseAxis, and HMD are all considered unsupported for binding purposes
        matches!(
            self,
            BindMain::Unsupported
                | BindMain::MouseWheelUp
                | BindMain::MouseWheelDown
                | BindMain::MouseAxis(_)
                | BindMain::HMD(_)
        )
    }
}

/// A single input bind: (modifiers) + main key, plus an optional activation-mode
/// reference (index into the ActivationArena).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bind {
    pub main: Option<BindMain>,
    pub modifiers: HashSet<Key>,

    /// Index into `ActivationArena` (`ActionBindings.activation`), if any.
    /// Replaces the old `Option<Uuid>`.
    pub activation_mode_idx: Option<usize>,

    /// True if explicitly unbound (no main key + no modifiers).
    pub is_unbound: bool,

    #[serde(default)]
    pub origin: BindOrigin,
}

#[derive(Debug, Clone)]
pub enum BindParseError {
    TooManyMainKeys {
        input: String,
        main_keys: Vec<String>,
    },
    NoInput,
}

impl PartialEq for Bind {
    fn eq(&self, other: &Self) -> bool {
        // Intentionally ignore activation_mode_idx and is_unbound for equality
        self.main == other.main && self.modifiers == other.modifiers
    }
}
impl Eq for Bind {}

impl Hash for Bind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.main.hash(state);

        // Hash modifiers in a deterministic order WITHOUT requiring Key: Ord
        let mut mods_as_strings: Vec<String> =
            self.modifiers.iter().map(|k| k.to_string()).collect();
        mods_as_strings.sort_unstable();
        for s in mods_as_strings {
            s.hash(state);
        }
    }
}

impl fmt::Display for Bind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Deterministic ordering of modifiers for display
        let mut mods: Vec<String> = self.modifiers.iter().map(|k| k.to_string()).collect();
        mods.sort_unstable();

        let mods_joined = mods.join("+");
        let main = self
            .main
            .as_ref()
            .map_or("<none>".to_string(), |k| k.to_string());

        if mods_joined.is_empty() {
            write!(f, "{main}")
        } else {
            write!(f, "{mods_joined}+{main}")
        }
    }
}

impl Bind {
    #[inline]
    pub fn is_executable(&self) -> bool {
        !self.is_unbound && self.main.is_some() && !self.main.as_ref().unwrap().is_unsupported()
    }

    #[inline]
    pub fn new(
        mainkey: Option<BindMain>,
        modifiers: HashSet<Key>,
        activation_mode_idx: Option<usize>,
    ) -> Self {
        let is_unbound = mainkey.is_none() && modifiers.is_empty();
        Bind {
            main: mainkey,
            modifiers,
            activation_mode_idx,
            is_unbound,
            origin: BindOrigin::User,
        }
    }

    pub fn generated(
        mainkey: BindMain,
        modifiers: HashSet<Key>,
        press_mode: Option<usize>,
    ) -> Self {
        Bind {
            main: Some(mainkey),
            modifiers,
            activation_mode_idx: press_mode,
            is_unbound: false,
            origin: BindOrigin::Generated,
        }
    }

    /// Parse a bind from a string like:
    ///   "lctrl+f", "LShift+A", "np_1", "kb1_lctrl+f", "" (empty means explicit unbind)
    ///
    /// `activation_mode_idx` is stored as-is (index into ActivationArena).
    pub fn from_string(
        input: &str,
        activation_mode_idx: Option<usize>,
    ) -> Result<Self, BindParseError> {
        // Empty → explicit unbound
        if input.trim().is_empty() {
            return Ok(Bind {
                main: None,
                modifiers: HashSet::new(),
                activation_mode_idx,
                is_unbound: true,
                origin: BindOrigin::User,
            });
        }

        // Strip only known device prefixes (don't break things like "np_1")
        let parts = strip_device_prefix(input);

        let segments: Vec<&str> = parts
            .split('+')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        let mut modifiers = HashSet::new();
        let mut main_keys: Vec<BindMain> = Vec::new();

        for seg in segments {
            let s = seg.to_ascii_lowercase();

            // 1) Wheel tokens
            match s.as_str() {
                "mwheel_up" | "mwheelup" | "wheel_up" | "mouse_wheel_up" => {
                    main_keys.push(BindMain::MouseWheelUp);
                    continue;
                }
                "mwheel_down" | "mwheeldown" | "wheel_down" | "mouse_wheel_down" => {
                    main_keys.push(BindMain::MouseWheelDown);
                    continue;
                }
                // 2) Mouse axes
                s if s.starts_with("maxis_") || s.starts_with("mouse_axis_") => {
                    let axis_name = s
                        .strip_prefix("maxis_")
                        .or_else(|| s.strip_prefix("mouse_axis_"))
                        .unwrap_or("unknown");
                    main_keys.push(BindMain::MouseAxis(axis_name.into()));
                    continue;
                }
                // 3) HMD axes
                s if s.starts_with("hmd_") => {
                    let hmd_name = s.strip_prefix("hmd_").unwrap_or("unknown");
                    main_keys.push(BindMain::HMD(hmd_name.into()));
                    continue;
                }
                _ => {}
            }

            // 3) Mouse buttons
            if let Some(m) = mouse_alias(&s) {
                main_keys.push(BindMain::Mouse(m));
                continue;
            }

            // 4) Keyboard
            if let Some(key) = Key::parse(&s) {
                if CANDIDATE_MODIFIERS.contains(&key) {
                    modifiers.insert(key);
                } else {
                    main_keys.push(BindMain::Key(key));
                }
                continue;
            }

            // Unknown segment
            return Err(BindParseError::NoInput);
        }

        match main_keys.len() {
            // Modifier-only bind: promote the single modifier to main key
            0 if modifiers.len() == 1 => {
                let mainkey = modifiers
                    .iter()
                    .next()
                    .cloned()
                    .ok_or(BindParseError::NoInput)?;
                Ok(Bind {
                    main: Some(BindMain::Key(mainkey)),
                    modifiers: HashSet::new(),
                    activation_mode_idx,
                    is_unbound: false,
                    origin: BindOrigin::User,
                })
            }
            1 => {
                let mainkey = main_keys.into_iter().next().unwrap();
                Ok(Bind {
                    main: Some(mainkey),
                    modifiers,
                    activation_mode_idx,
                    is_unbound: false,
                    origin: BindOrigin::User,
                })
            }
            _ => Err(BindParseError::TooManyMainKeys {
                input: input.to_string(),
                main_keys: main_keys.iter().map(|k| k.to_string()).collect(),
            }),
        }
    }
}

// Only strip prefixes we actually expect from SC XML like "kb1_", "mo1_", "gp1_"
fn strip_device_prefix(s: &str) -> &str {
    const PREFIXES: &[&str] = &[
        "kb1_", "kb2_", "kb_", // keyboard instances (be liberal)
        "mo1_", "mo2_", "mo_", // mouse instances
        "gp1_", "gp2_", "gp_", // gamepad
        "js1_", "js2_", "js_", // joystick (if it ever shows up)
    ];
    for p in PREFIXES {
        if let Some(end) = s.strip_prefix(p) {
            return end;
        }
    }
    s
}

fn mouse_alias(seg: &str) -> Option<MouseButton> {
    let s = seg.trim().to_ascii_lowercase();

    // Handle "mouse<N>" and "mouse<N>_<M>" by taking the last number
    if let Some(rest) = s.strip_prefix("mouse") {
        let last_num = rest
            .split('_')
            .filter_map(|p| p.parse::<u16>().ok())
            .next_back();

        if let Some(n) = last_num {
            return Some(match n {
                1 => MouseButton::Left,
                2 => MouseButton::Right,
                3 => MouseButton::Middle,
                4 => MouseButton::X(1),
                5 => MouseButton::X(2),
                m if m >= 6 => MouseButton::X(m - 3), // crude mapping for higher numbers
                _ => MouseButton::Left,
            });
        }
    }

    match s.as_str() {
        "mouse1" | "lmb" | "mouse_left" => Some(MouseButton::Left),
        "mouse2" | "rmb" | "mouse_right" => Some(MouseButton::Right),
        "mouse3" | "mmb" | "mouse_middle" => Some(MouseButton::Middle),
        "mouse4" | "mb4" | "x1" | "mouse_x1" => Some(MouseButton::X(1)),
        "mouse5" | "mb5" | "x2" | "mouse_x2" => Some(MouseButton::X(2)),
        _ => None,
    }
}

fn mouse_to_str(btn: MouseButton) -> String {
    match btn {
        MouseButton::Left => "mouse1".into(),
        MouseButton::Right => "mouse2".into(),
        MouseButton::Middle => "mouse3".into(),
        MouseButton::X(1) => "mouse4".into(),
        MouseButton::X(2) => "mouse5".into(),
        MouseButton::X(n) => format!("mouse{}", n + 3),
    }
}
