use roxmltree::Node;
use serde::{ Deserialize, Serialize };
#[cfg(windows)]
use streamdeck_lib::prelude::*;
use crate::bindings::{
    activation_mode::{ ActivationMode, ActivationArena },
    bind::BindParseError,
    binds::Binds,
    helpers::get_translation,
    str_intern::{ intern },
};
use std::collections::HashMap;
#[cfg(windows)]
use std::{ sync::Arc, time::Duration };

#[derive(Debug)]
pub enum ActionBindingParseError {
    MissingName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBinding {
    pub action_id: Arc<str>,
    pub action_name: Arc<str>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_label: Option<Arc<str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_description: Option<Arc<str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<Arc<str>>,

    pub default_binds: Binds,
    pub custom_binds: Option<Binds>,

    /// Arena index into the shared activation modes (deduped).
    /// (Action-level fallback when a bind doesn't have its own.)
    pub activation_mode: Option<usize>,
}

impl ActionBinding {
    #[inline]
    fn non_empty_attr(node: Node, key: &str) -> Option<String> {
        node.attribute(key).and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        })
    }

    pub fn from_node(
        node: Node,
        action_map_name: &str,
        activation_arena: &mut ActivationArena
    ) -> Result<(Self, Vec<BindParseError>), ActionBindingParseError> {
        let name = node.attribute("name").ok_or(ActionBindingParseError::MissingName)?.to_string();

        let action_id = intern(format!("{action_map_name}.{name}"));
        let action_name = intern(name);
        let ui_label = Self::non_empty_attr(node, "UILabel").map(intern);
        let ui_description = Self::non_empty_attr(node, "UIDescription").map(intern);
        let category = Self::non_empty_attr(node, "Category").map(intern);

        // Binds resolve their own bind-level activation modes into arena indices.
        let (default_binds, bind_errors) = Binds::from_node(node, activation_arena);

        // Action-level activation mode (fallback for binds that don’t specify a mode)
        let action_level_mode = resolve_mode_idx(node, None, activation_arena);

        Ok((
            ActionBinding {
                action_id,
                action_name,
                ui_label,
                ui_description,
                category,
                default_binds,
                custom_binds: None,
                activation_mode: action_level_mode,
            },
            bind_errors,
        ))
    }

    pub fn get_label(&self, translations: &HashMap<String, String>) -> String {
        let key = self.ui_label.as_deref().unwrap_or(&self.action_name);
        get_translation(key, translations).to_string()
    }

    /// Human-friendly summary of binds (keyboard + mouse). `None` only if *both* are empty.
    pub fn get_binds_label(&self) -> Option<String> {
        let binds = self.custom_binds.as_ref().unwrap_or(&self.default_binds);

        let mut parts: Vec<String> = Vec::new();

        let kb = binds.keyboard
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>();
        if !kb.is_empty() {
            parts.push(kb.join(", "));
        }

        let mouse = binds.mouse
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>();
        if !mouse.is_empty() {
            parts.push(mouse.join(", "));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" | "))
        }
    }

    // ---------------- Windows send path ----------------

    /// Windows synth of the first bind (prefers keyboard; falls back to mouse),
    /// using an arena of activation modes for resolution.
    #[cfg(windows)]
    pub fn simulate_with_modes(
        &self,
        logger: Arc<dyn ActionLog>,
        hold_duration_override: Option<Duration>,
        is_down_override: Option<bool>,
        modes: &crate::bindings::activation_mode::ActivationArena
    ) -> Result<(), String> {
        use streamdeck_lib::input::dsl;
        use streamdeck_lib::input::{ InputStep, Key, MouseButton, InputSynth, WinSynth };
        use crate::bindings::bind::BindMain;

        // 0) Choose a bind: prefer keyboard, else mouse
        let bind = {
            let src = self.custom_binds.as_ref().unwrap_or(&self.default_binds);
            let kb = src.keyboard
                .iter()
                .find(|b| !b.is_unbound)
                .cloned();
            kb
                .or_else(||
                    src.mouse
                        .iter()
                        .find(|b| !b.is_unbound)
                        .cloned()
                )
                .ok_or_else(|| "No keyboard or mouse bind found".to_string())?
        };

        // 1) Resolve activation mode index: bind-level first, then action-level
        let am_ix = bind.activation_mode_idx
            .or(self.activation_mode)
            .ok_or_else(|| "No activation mode available".to_string())?;
        let mode = modes.get(am_ix).ok_or("Activation mode index out of range")?;

        // 2) Sort modifiers stably (by scancode when available)
        let mut mods: Vec<Key> = bind.modifiers.iter().copied().collect();
        mods.sort_by_key(|k|
            k
                .to_scan()
                .map(|s| (0u8, s.code))
                .unwrap_or((1, 0))
        );

        // Helpful debug: show how each modifier resolves
        for &m in &mods {
            if let Some(s) = m.to_scan() {
                debug!(
                    logger,
                    "modifier {:?} -> Scan {{ code: 0x{:X}, extended: {} }}",
                    m,
                    s.code,
                    s.extended
                );
            } else {
                // Not expected on Windows, except for unmapped keys
                debug!(logger, "modifier {:?} has no scancode mapping", m);
            }
        }

        // 3) Helpers
        let synth = WinSynth::new();

        // Best-effort: send all steps; if anything fails we still try to finish.
        // Returns Ok if *all* steps succeeded; otherwise returns the first error string.
        let send_steps = |steps: &[InputStep]| -> Result<(), String> {
            let mut first_err: Option<String> = None;
            for s in steps {
                if let Err(e) = synth.send_step(s) {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
            }
            first_err.map_or(Ok(()), Err)
        };

        // Utility: best-effort release ALL these modifiers no matter what
        let release_mods = |mods: &[Key]| {
            for &m in mods.iter().rev() {
                if let Some(s) = m.to_step_up() {
                    let _ = synth.send_step(&s); // ignore error; recovery path
                }
            }
        };

        // Safety wrapper: send `steps`, and (optionally) always try a final release of modifiers.
        // Use this for all "balanced" flows (tap/chord/hold, releases, etc.).
        // For explicit "down-only" overrides, pass `release_safety = false`.
        let send_with_safety = |steps: Vec<InputStep>, release_safety: bool| -> Result<(), String> {
            let res = send_steps(&steps);
            if release_safety {
                // Even if steps were balanced, an intermediate failure could leave a mod down.
                // Sending extra KeyUp for a not-down key is harmless on Windows.
                release_mods(&mods);
            }
            res
        };

        let compute_hold_ms = || -> u64 {
            if let Some(ov) = hold_duration_override {
                return (ov.as_millis() as u64).saturating_add(50);
            }
            let base_ms = if let Some(th) = mode.press_trigger_threshold {
                if th > 0.0 { (th * 1000.0) as u64 } else { 260 }
            } else if let Some(d) = mode.hold_trigger_delay {
                if d > 0.0 { (d * 1000.0) as u64 } else { 260 }
            } else {
                260
            };
            base_ms.saturating_add(50)
        };

        // Mouse helpers (balanced sequences)
        let mouse_chord = |mods: &[Key], btn: MouseButton| -> Vec<InputStep> {
            let mut v = Vec::new();
            for &m in mods {
                if let Some(s) = m.to_step_down() {
                    v.push(s);
                }
            }
            v.push(InputStep::MouseDown(btn));
            v.push(InputStep::MouseUp(btn));
            for &m in mods.iter().rev() {
                if let Some(s) = m.to_step_up() {
                    v.push(s);
                }
            }
            v
        };
        let mouse_hold = |mods: &[Key], btn: MouseButton, ms: u64| -> Vec<InputStep> {
            let mut v = Vec::new();
            for &m in mods {
                if let Some(s) = m.to_step_down() {
                    v.push(s);
                }
            }
            v.push(InputStep::MouseDown(btn));
            v.push(dsl::sleep_ms(ms));
            v.push(InputStep::MouseUp(btn));
            for &m in mods.iter().rev() {
                if let Some(s) = m.to_step_up() {
                    v.push(s);
                }
            }
            v
        };

        // 4) Behavior by main kind
        match bind.main.ok_or_else(|| "Bind has no main input".to_string())? {
            BindMain::Key(main_key) => {
                debug!(
                    logger,
                    "simulate(key): id={} main={:?} mods={:?} mode={:?}",
                    self.action_id,
                    main_key,
                    mods,
                    mode
                );

                // Overrides
                if let Some(ov) = is_down_override {
                    if ov {
                        // DOWN-ONLY: press mods then key down; DO NOT auto-release
                        let mut steps = Vec::new();
                        for m in &mods {
                            if let Some(s) = m.to_step_down() {
                                steps.push(s);
                            }
                        }
                        if let Some(s) = main_key.to_step_down() {
                            steps.push(s);
                        }
                        return send_with_safety(steps, /*release_safety=*/ false);
                    } else {
                        // UP-ONLY: key up then release mods; still do safety release
                        let mut steps = Vec::new();
                        if let Some(s) = main_key.to_step_up() {
                            steps.push(s);
                        }
                        for m in mods.iter().rev() {
                            if let Some(s) = m.to_step_up() {
                                steps.push(s);
                            }
                        }
                        return send_with_safety(steps, /*release_safety=*/ true);
                    }
                }

                // Multi-tap
                let taps = mode.multi_tap.max(1) as usize;
                if taps >= 2 {
                    let mut steps = Vec::new();
                    for i in 0..taps {
                        steps.extend(dsl::chord(&mods, main_key));
                        if i + 1 < taps {
                            steps.push(dsl::sleep_ms(25));
                        }
                    }
                    return send_with_safety(steps, /*release_safety=*/ true);
                }

                // Hold?
                let wants_hold = mode.on_hold || mode.press_trigger_threshold.unwrap_or(0.0) > 0.0;
                if wants_hold {
                    return send_with_safety(
                        dsl::hold(&mods, main_key, compute_hold_ms()),
                        /*release_safety=*/ true
                    );
                }

                // Release-only → chord fallback
                if mode.on_release && !mode.on_press && !mode.on_hold {
                    return send_with_safety(dsl::chord(&mods, main_key), /*release_safety=*/ true);
                }

                // Default press (balanced)
                send_with_safety(dsl::chord(&mods, main_key), /*release_safety=*/ true)
            }

            BindMain::Mouse(btn) => {
                debug!(
                    logger,
                    "simulate(mouse): id={} btn={:?} mods={:?} mode={:?}",
                    self.action_id,
                    btn,
                    mods,
                    mode
                );

                // Overrides
                if let Some(ov) = is_down_override {
                    if ov {
                        // DOWN-ONLY: press mods then mouse down; DO NOT auto-release
                        let mut steps = Vec::new();
                        for m in &mods {
                            if let Some(s) = m.to_step_down() {
                                steps.push(s);
                            }
                        }
                        steps.push(InputStep::MouseDown(btn));
                        return send_with_safety(steps, /*release_safety=*/ false);
                    } else {
                        // UP-ONLY: mouse up then release mods; still do safety release
                        let mut steps = Vec::new();
                        steps.push(InputStep::MouseUp(btn));
                        for m in mods.iter().rev() {
                            if let Some(s) = m.to_step_up() {
                                steps.push(s);
                            }
                        }
                        return send_with_safety(steps, /*release_safety=*/ true);
                    }
                }

                // Multi-tap
                let taps = mode.multi_tap.max(1) as usize;
                if taps >= 2 {
                    let mut steps = Vec::new();
                    for i in 0..taps {
                        steps.extend(mouse_chord(&mods, btn));
                        if i + 1 < taps {
                            steps.push(dsl::sleep_ms(25));
                        }
                    }
                    return send_with_safety(steps, /*release_safety=*/ true);
                }

                // Hold?
                let wants_hold = mode.on_hold || mode.press_trigger_threshold.unwrap_or(0.0) > 0.0;
                if wants_hold {
                    return send_with_safety(
                        mouse_hold(&mods, btn, compute_hold_ms()),
                        /*release_safety=*/ true
                    );
                }

                // Release-only → chord fallback
                if mode.on_release && !mode.on_press && !mode.on_hold {
                    return send_with_safety(mouse_chord(&mods, btn), /*release_safety=*/ true);
                }

                // Default click (balanced)
                send_with_safety(mouse_chord(&mods, btn), /*release_safety=*/ true)
            }
        }
    }

    /// Convenience using a full snapshot (for arena access).
    #[cfg(windows)]
    pub fn simulate_using(
        &self,
        logger: Arc<dyn ActionLog>,
        hold_duration_override: Option<Duration>,
        is_down_override: Option<bool>,
        bindings: &crate::bindings::action_bindings::ActionBindings
    ) -> Result<(), String> {
        self.simulate_with_modes(
            logger,
            hold_duration_override,
            is_down_override,
            &bindings.activation
        )
    }

    /// Non-Windows placeholder
    #[cfg(not(windows))]
    pub fn simulate_with_modes(
        &self,
        _logger: Arc<dyn ActionLog>,
        _hold_duration_override: Option<Duration>,
        _is_down_override: Option<bool>,
        _modes: &[ActivationMode]
    ) -> Result<(), String> {
        Err("simulate is only implemented on Windows".into())
    }

    #[cfg(not(windows))]
    pub fn simulate_using(
        &self,
        _logger: Arc<dyn ActionLog>,
        _hold_duration_override: Option<Duration>,
        _is_down_override: Option<bool>,
        _bindings: &crate::bindings::action_bindings::ActionBindings
    ) -> Result<(), String> {
        Err("simulate is only implemented on Windows".into())
    }
}

/// Local helper: resolve an activation mode to an arena index.
///
/// Order:
/// 1) If `activationMode="Name"` is present:
///    - return existing arena index if a named mode exists
///    - else define from this node’s attrs (or fallback’s), name it, insert+return idx
/// 2) Else, if node has inline activation attrs, insert anonymous mode and return idx
/// 3) Else, if fallback has attrs, insert anonymous fallback mode and return idx
/// 4) Else, None
fn resolve_mode_idx(
    node: Node,
    fallback: Option<Node>,
    arena: &mut ActivationArena
) -> Option<usize> {
    // Named reference
    if let Some(mode_name) = node.attribute("activationMode") {
        if let Some(idx) = arena.find_by_name(mode_name) {
            return Some(idx);
        }

        let candidate = if ActivationMode::has_valid_attributes(node) {
            let mut m = ActivationMode::from_node(node, true);
            m.name = Some(mode_name.to_string());
            m
        } else if let Some(f) = fallback.filter(|n| ActivationMode::has_valid_attributes(*n)) {
            let mut m = ActivationMode::from_node(f, true);
            m.name = Some(mode_name.to_string());
            m
        } else {
            return None; // name given but nowhere to define it
        };

        return Some(arena.insert_or_get_mode(candidate));
    }

    // Inline anonymous
    if ActivationMode::has_valid_attributes(node) {
        return Some(arena.insert_or_get_mode(ActivationMode::from_node(node, false)));
    }

    // Fallback anonymous
    if let Some(f) = fallback.filter(|n| ActivationMode::has_valid_attributes(*n)) {
        return Some(arena.insert_or_get_mode(ActivationMode::from_node(f, false)));
    }

    None
}
