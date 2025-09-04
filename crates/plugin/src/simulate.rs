use std::{sync::Arc, time::Duration};
use streamdeck_lib::input::{InputStep, InputSynth, MouseButton, WinSynth, dsl};
use streamdeck_lib::prelude::*;

use streamdeck_sc_core::bindings::{
    action_binding::ActionBinding,
    activation_mode::ActivationArena,
    bind::{BindMain, Key as CoreKey},
};

pub trait SimulateExt {
    fn simulate_with_modes(
        &self,
        logger: Arc<dyn ActionLog>,
        hold_duration_override: Option<Duration>,
        is_down_override: Option<bool>,
        modes: &ActivationArena,
    ) -> Result<(), String>;

    fn simulate_using(
        &self,
        logger: Arc<dyn ActionLog>,
        hold_duration_override: Option<Duration>,
        is_down_override: Option<bool>,
        bindings: &streamdeck_sc_core::bindings::action_bindings::ActionBindings,
    ) -> Result<(), String>;
}

impl SimulateExt for ActionBinding {
    fn simulate_with_modes(
        &self,
        _logger: Arc<dyn ActionLog>,
        hold_duration_override: Option<Duration>,
        is_down_override: Option<bool>,
        modes: &ActivationArena,
    ) -> Result<(), String> {
        // ---- pick the first runnable bind (prefers keyboard) ----
        let pick_first_runnable = |binds: &streamdeck_sc_core::bindings::binds::Binds| {
            binds
                .keyboard
                .iter()
                .chain(binds.mouse.iter())
                .filter(|b| !b.is_unbound && b.is_executable())
                .cloned()
                .next()
        };

        let bind = if let Some(cb) = self.custom_binds.as_ref() {
            pick_first_runnable(cb).or_else(|| pick_first_runnable(&self.default_binds))
        } else {
            pick_first_runnable(&self.default_binds)
        }
        .ok_or_else(|| "No executable bind found (only wheel/axis/HMD or unbound)".to_string())?;

        // ---- activation mode ----
        let am_ix = bind
            .activation_mode_idx
            .or(self.activation_mode)
            .ok_or_else(|| "No activation mode available".to_string())?;
        let mode = modes
            .get(am_ix)
            .ok_or("Activation mode index out of range")?;

        // ---- stable order for modifiers ----
        let mut mods: Vec<CoreKey> = bind.modifiers.iter().copied().collect();
        mods.sort_by_key(|k| k.to_scan().map(|s| (0u8, s.code)).unwrap_or((1, 0)));

        let synth = WinSynth::new();

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

        let release_mods = |mods: &[CoreKey]| {
            for &m in mods.iter().rev() {
                if let Some(s) = m.to_step_up() {
                    let _ = synth.send_step(&s);
                }
            }
        };

        let send_with_safety =
            |steps: Vec<InputStep>, release_safety: bool| -> Result<(), String> {
                let res = send_steps(&steps);
                if release_safety {
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

        let mouse_chord = |mods: &[CoreKey], btn: MouseButton| -> Vec<InputStep> {
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

        let mouse_hold = |mods: &[CoreKey], btn: MouseButton, ms: u64| -> Vec<InputStep> {
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

        match bind
            .main
            .ok_or_else(|| "Bind has no main input".to_string())?
        {
            BindMain::Key(main_key) => {
                if let Some(is_down) = is_down_override {
                    if is_down {
                        let mut steps = Vec::new();
                        for m in &mods {
                            if let Some(s) = m.to_step_down() {
                                steps.push(s);
                            }
                        }
                        if let Some(s) = main_key.to_step_down() {
                            steps.push(s);
                        }
                        return send_with_safety(steps, false);
                    } else {
                        let mut steps = Vec::new();
                        if let Some(s) = main_key.to_step_up() {
                            steps.push(s);
                        }
                        for m in mods.iter().rev() {
                            if let Some(s) = m.to_step_up() {
                                steps.push(s);
                            }
                        }
                        return send_with_safety(steps, true);
                    }
                }

                let taps = mode.multi_tap.max(1) as usize;
                if taps >= 2 {
                    let mut steps = Vec::new();
                    for i in 0..taps {
                        steps.extend(dsl::chord(&mods, main_key));
                        if i + 1 < taps {
                            steps.push(dsl::sleep_ms(25));
                        }
                    }
                    return send_with_safety(steps, true);
                }

                let wants_hold = mode.on_hold || mode.press_trigger_threshold.unwrap_or(0.0) > 0.0;
                if wants_hold {
                    return send_with_safety(dsl::hold(&mods, main_key, compute_hold_ms()), true);
                }

                if mode.on_release && !mode.on_press && !mode.on_hold {
                    return send_with_safety(dsl::chord(&mods, main_key), true);
                }

                send_with_safety(dsl::chord(&mods, main_key), true)
            }

            BindMain::Mouse(btn) => {
                if let Some(is_down) = is_down_override {
                    if is_down {
                        let mut steps = Vec::new();
                        for m in &mods {
                            if let Some(s) = m.to_step_down() {
                                steps.push(s);
                            }
                        }
                        steps.push(InputStep::MouseDown(btn));
                        return send_with_safety(steps, false);
                    } else {
                        let mut steps = Vec::new();
                        steps.push(InputStep::MouseUp(btn));
                        for m in mods.iter().rev() {
                            if let Some(s) = m.to_step_up() {
                                steps.push(s);
                            }
                        }
                        return send_with_safety(steps, true);
                    }
                }

                let taps = mode.multi_tap.max(1) as usize;
                if taps >= 2 {
                    let mut steps = Vec::new();
                    for i in 0..taps {
                        steps.extend(mouse_chord(&mods, btn));
                        if i + 1 < taps {
                            steps.push(dsl::sleep_ms(25));
                        }
                    }
                    return send_with_safety(steps, true);
                }

                let wants_hold = mode.on_hold || mode.press_trigger_threshold.unwrap_or(0.0) > 0.0;
                if wants_hold {
                    return send_with_safety(mouse_hold(&mods, btn, compute_hold_ms()), true);
                }

                if mode.on_release && !mode.on_press && !mode.on_hold {
                    return send_with_safety(mouse_chord(&mods, btn), true);
                }

                send_with_safety(mouse_chord(&mods, btn), true)
            }

            _ => Err("Bind main is not a key or mouse button".to_string()),
        }
    }

    fn simulate_using(
        &self,
        logger: Arc<dyn ActionLog>,
        hold_duration_override: Option<Duration>,
        is_down_override: Option<bool>,
        bindings: &streamdeck_sc_core::bindings::action_bindings::ActionBindings,
    ) -> Result<(), String> {
        self.simulate_with_modes(
            logger,
            hold_duration_override,
            is_down_override,
            &bindings.activation,
        )
    }
}
