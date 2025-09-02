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
    /// NOTE:
    /// - Explicit `unbound` entries are *kept* (b.is_unbound == true) so callers can distinguish
    ///   “explicitly clear this device” from “no change”.
    /// - We no longer drop wheel/axis/HMD: they are parsed to MouseWheelUp/Down/Unsupported.
    pub fn from_node(
        action_node: roxmltree::Node,
        activation_modes: &mut ActivationArena,
    ) -> (Self, Vec<BindParseError>) {
        let mut keyboard = Vec::new();
        let mut mouse = Vec::new();
        let mut errors = Vec::new();

        // Route *all* parsed binds, including explicit unbound, so the caller can tell intent.
        let mut route = |b: Bind| match b.main {
            // Wheel/Unsupported do not imply mouse vs keyboard; treat as keyboard side to match SC’s XML,
            // BUT this only affects where they show up in our struct, not runtime behavior.
            Some(BindMain::Mouse(_)) => mouse.push(b),
            // Some(BindMain::MouseWheelUp) | Some(BindMain::MouseWheelDown) => mouse.push(b),
            _ => keyboard.push(b),
        };

        // ---- flat attributes ----------------------------------------------------
        for attr_name in ["keyboard", "mouse"] {
            if let Some(raw) = action_node.attribute(attr_name) {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let mode = ActivationMode::resolve(action_node, None, activation_modes);
                match Bind::from_string(trimmed, mode) {
                    Ok(b) => route(b),
                    Err(e) => errors.push(e),
                }
            }
        }

        // ---- nested device nodes ------------------------------------------------
        for node in action_node
            .children()
            .filter(|n| n.is_element() && (n.has_tag_name("keyboard") || n.has_tag_name("mouse")))
        {
            if let Some(raw) = node.attribute("input") {
                let trimmed = raw.trim();
                if !trimmed.is_empty() {
                    let mode = ActivationMode::resolve(node, Some(action_node), activation_modes);
                    match Bind::from_string(trimmed, mode) {
                        Ok(b) => route(b),
                        Err(e) => errors.push(e),
                    }
                }
            }

            for input in node
                .children()
                .filter(|n| n.is_element() && n.has_tag_name("inputdata"))
            {
                if let Some(raw) = input.attribute("input") {
                    let trimmed = raw.trim();
                    if trimmed.is_empty() {
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
