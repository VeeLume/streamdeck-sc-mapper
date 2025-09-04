use serde::{Deserialize, Serialize};

use crate::bindings::activation_mode::{ActivationArena, ActivationMode};
use crate::bindings::bind::{Bind, BindMain, BindParseError};

/// Keyboard/mouse binds for a single `<action>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Binds {
    pub keyboard: Vec<Bind>,
    pub mouse: Vec<Bind>,
}

impl Binds {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// True if any bind is present that isn’t explicitly unbound.
    #[inline]
    pub fn has_active_binds(&self) -> bool {
        self.keyboard.iter().any(|b| !b.is_unbound) || self.mouse.iter().any(|b| !b.is_unbound)
    }

    /// Iterate all binds by reference, keyboard first then mouse.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Bind> {
        self.keyboard.iter().chain(self.mouse.iter())
    }

    /// Iterate all binds mutably, keyboard first then mouse.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Bind> {
        self.keyboard.iter_mut().chain(self.mouse.iter_mut())
    }

    /// Return an owning iterator (clones).
    #[inline]
    pub fn all_binds(&self) -> impl Iterator<Item = Bind> + '_ {
        self.iter().cloned()
    }

    #[inline]
    pub fn push_keyboard(&mut self, b: Bind) {
        self.keyboard.push(b);
    }

    #[inline]
    pub fn push_mouse(&mut self, b: Bind) {
        self.mouse.push(b);
    }

    /// Parse binds for an `<action>` node, resolving activation modes into an arena (indices).
    ///
    /// Notes:
    /// - Explicit `unbound` entries are kept (`b.is_unbound == true`) so callers can distinguish
    ///   “explicitly clear this device” from “no change”.
    /// - We still route wheel/axis/HMD tokens into the keyboard side by default (historical choice),
    ///   except explicit mouse buttons which go to `mouse`.
    pub fn from_node(
        action_node: roxmltree::Node,
        activation_modes: &mut ActivationArena,
    ) -> (Self, Vec<BindParseError>) {
        let mut out = Binds::default();
        let mut errors = Vec::new();

        // Route *all* parsed binds, including explicit unbound, so the caller can tell intent.
        let mut route = |b: Bind| match b.main {
            Some(BindMain::Mouse(_)) => out.mouse.push(b),
            _ => out.keyboard.push(b),
        };

        // ---- flat attributes: <action keyboard="..." mouse="..."> ----
        parse_device_attr(
            action_node,
            "keyboard",
            None,
            activation_modes,
            &mut route,
            &mut errors,
        );
        parse_device_attr(
            action_node,
            "mouse",
            None,
            activation_modes,
            &mut route,
            &mut errors,
        );

        // ---- nested device nodes: <action><keyboard input="..."><inputdata .../></keyboard> ... ----
        for node in action_node
            .children()
            .filter(|n| n.is_element() && (n.has_tag_name("keyboard") || n.has_tag_name("mouse")))
        {
            // direct attribute on the device node
            parse_device_attr(
                node,
                "input",
                Some(action_node),
                activation_modes,
                &mut route,
                &mut errors,
            );

            // child <inputdata input="...">
            for input in node
                .children()
                .filter(|n| n.is_element() && n.has_tag_name("inputdata"))
            {
                // Prefer mode resolved at <inputdata>, fall back to the device node, then action node.
                let mode = ActivationMode::resolve(input, Some(node), activation_modes)
                    .or_else(|| ActivationMode::resolve(node, Some(action_node), activation_modes))
                    .or_else(|| ActivationMode::resolve(action_node, None, activation_modes));

                if let Some(raw) = input.attribute("input") {
                    let trimmed = raw.trim();
                    if !trimmed.is_empty() {
                        match Bind::from_string(trimmed, mode) {
                            Ok(b) => route(b),
                            Err(e) => errors.push(e),
                        }
                    }
                }
            }
        }

        (out, errors)
    }
}

/// Helper: parse a single attribute from `node` and route a `Bind` if present.
/// If `parent_for_mode` is Some, the parent participates in activation-mode resolution.
fn parse_device_attr<'a, F>(
    node: roxmltree::Node<'a, 'a>,
    attr: &str,
    parent_for_mode: Option<roxmltree::Node<'a, 'a>>,
    activation_modes: &mut ActivationArena,
    route: &mut F,
    errors: &mut Vec<BindParseError>,
) where
    F: FnMut(Bind),
{
    if let Some(raw) = node.attribute(attr) {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return;
        }

        // Try resolving at this node first, then optionally at parent, then no mode.
        let mode = ActivationMode::resolve(node, parent_for_mode, activation_modes).or_else(|| {
            parent_for_mode.and_then(|p| ActivationMode::resolve(p, None, activation_modes))
        });

        match Bind::from_string(trimmed, mode) {
            Ok(b) => route(b),
            Err(e) => errors.push(e),
        }
    }
}
