use serde::{ Deserialize, Serialize };

use crate::action_binds::activation_mode::ActivationMode;
use crate::action_binds::bind::Bind;
use crate::action_binds::bind::BindParseError;

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

    pub fn from_node(
        action_node: roxmltree::Node,
        activation_modes: &[ActivationMode]
    ) -> (Self, Vec<BindParseError>) {
        let mut errors = Vec::new();

        let (keyboard, keyboard_errors) = Self::parse_device_binds(
            "keyboard",
            action_node,
            activation_modes
        );
        let (mouse, mouse_errors) = Self::parse_device_binds(
            "mouse",
            action_node,
            activation_modes
        );

        errors.extend(keyboard_errors);
        errors.extend(mouse_errors);

        (
            Binds {
                keyboard,
                mouse,
            },
            errors,
        )
    }

    fn parse_device_binds(
        device_name: &str,
        action_node: roxmltree::Node,
        activation_modes: &[ActivationMode]
    ) -> (Vec<Bind>, Vec<BindParseError>) {
        let mut binds = Vec::new();
        let mut errors = Vec::new();

        // --- Case 1 & 2: flat bind <action keyboard="...">
        if let Some(raw) = action_node.attribute(device_name) {
            let mode = ActivationMode::resolve(action_node, None, activation_modes);
            match Bind::from_string(raw, mode) {
                Ok(b) => binds.push(b),
                Err(e) => errors.push(e),
            }
        }

        // --- Case 3 & 4: nested node with input attr or <inputdata>
        for node in action_node
            .children()
            .filter(|n| n.is_element() && n.has_tag_name(device_name)) {
            // Case 3: <keyboard input="e" />
            if let Some(raw) = node.attribute("input") {
                let mode = ActivationMode::resolve(node, Some(action_node), activation_modes);
                match Bind::from_string(raw, mode) {
                    Ok(b) => binds.push(b),
                    Err(e) => errors.push(e),
                }
            }

            // Case 4: <inputdata input="..." />
            for input in node.children().filter(|n| n.is_element() && n.has_tag_name("inputdata")) {
                if let Some(raw) = input.attribute("input") {
                    let mode = ActivationMode::resolve(action_node, None, activation_modes);
                    match Bind::from_string(raw, mode) {
                        Ok(b) => binds.push(b),
                        Err(e) => errors.push(e),
                    }
                }
            }
        }

        (binds, errors)
    }
}
