use serde::{Deserialize, Serialize};

use crate::bindings::activation_mode::{ActivationArena, ActivationMode};
use crate::bindings::bind::{Bind, BindMain, BindParseError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Binds {
    pub keyboard: Vec<Bind>,
    pub mouse: Vec<Bind>,
}

impl Binds {
    pub fn new() -> Self {
        Binds {
            keyboard: Vec::new(),
            mouse: Vec::new(),
        }
    }

    /// Returns true if there are any active binds (not unbound) in either keyboard or mouse.
    pub fn has_active_binds(&self) -> bool {
        self.keyboard.iter().any(|b| !b.is_unbound) || self.mouse.iter().any(|b| !b.is_unbound)
    }

    pub fn all_binds(&self) -> impl Iterator<Item = Bind> + '_ {
        self.keyboard.iter().chain(self.mouse.iter()).cloned()
    }

    /// Parse binds for an <action> node, resolving activation modes into an arena (indices).
    pub fn from_node(
        action_node: roxmltree::Node,
        activation_modes: &mut ActivationArena,
    ) -> (Self, Vec<BindParseError>) {
        let mut keyboard = Vec::new();
        let mut mouse = Vec::new();
        let mut errors = Vec::new();

        #[inline]
        fn contains_ignored_input(raw: &str) -> bool {
            raw.split('+')
                .map(|s| s.trim().to_ascii_lowercase())
                .any(|tok| {
                    matches!(
                        tok.as_str(),
                        // wheel
                        "mwheel" |
                            "mwheel_up" |
                            "mwheel_down" |
                            // axes
                            "maxis_x" |
                            "maxis_y" |
                            "maxis_z" |
                            "maxis_rx" |
                            "maxis_ry" |
                            "maxis_rz" |
                            // hmd
                            "hmd_roll" |
                            "hmd_pitch" |
                            "hmd_yaw"
                    )
                })
        }

        let mut route = |b: Bind| {
            if b.is_unbound {
                return;
            }
            match b.main {
                Some(BindMain::Mouse(_)) => mouse.push(b),
                _ => keyboard.push(b),
            }
        };

        // flat attributes
        for attr_name in ["keyboard", "mouse"] {
            if let Some(raw) = action_node.attribute(attr_name) {
                let trimmed = raw.trim();
                if contains_ignored_input(trimmed) {
                    // e.g. "ralt+mwheel_down" (not supported) — skip quietly
                    continue;
                }
                let mode = ActivationMode::resolve(action_node, None, activation_modes);
                match Bind::from_string(trimmed, mode) {
                    Ok(b) => route(b),
                    Err(e) => errors.push(e),
                }
            }
        }

        // nested device nodes
        for node in action_node
            .children()
            .filter(|n| n.is_element() && (n.has_tag_name("keyboard") || n.has_tag_name("mouse")))
        {
            if let Some(raw) = node.attribute("input") {
                let trimmed = raw.trim();
                if contains_ignored_input(trimmed) {
                    continue;
                }

                let mode = ActivationMode::resolve(node, Some(action_node), activation_modes);
                match Bind::from_string(trimmed, mode) {
                    Ok(b) => route(b),
                    Err(e) => errors.push(e),
                }
            }

            for input in node
                .children()
                .filter(|n| n.is_element() && n.has_tag_name("inputdata"))
            {
                if let Some(raw) = input.attribute("input") {
                    let trimmed = raw.trim();
                    if contains_ignored_input(trimmed) {
                        continue;
                    }

                    let mode = ActivationMode::resolve(input, Some(node), activation_modes)
                        .or_else(|| ActivationMode::resolve(action_node, None, activation_modes));

                    match Bind::from_string(trimmed, mode) {
                        Ok(b) => route(b),
                        Err(e) => errors.push(e),
                    }
                }
            }
        }

        (Binds { keyboard, mouse }, errors)
    }
}
