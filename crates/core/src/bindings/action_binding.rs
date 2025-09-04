use crate::bindings::{
    activation_mode::{ActivationArena, ActivationMode},
    bind::BindParseError,
    binds::Binds,
    str_intern::intern,
    translations::get_translation,
};
use roxmltree::Node;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_binds: Option<Binds>,

    /// Arena index into the shared activation modes (deduped).
    /// (Action-level fallback when a bind doesn't have its own.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

    /// Parse an `<action>` node into an `ActionBinding` and collect bind parse errors.
    pub fn from_node(
        node: Node,
        action_map_name: &str,
        activation_arena: &mut ActivationArena,
    ) -> Result<(Self, Vec<BindParseError>), ActionBindingParseError> {
        let name = node
            .attribute("name")
            .ok_or(ActionBindingParseError::MissingName)?
            .to_string();

        let action_id = intern(format!("{action_map_name}.{name}"));
        let action_name = intern(name);
        let ui_label = Self::non_empty_attr(node, "UILabel").map(intern);
        let ui_description = Self::non_empty_attr(node, "UIDescription").map(intern);
        let category = Self::non_empty_attr(node, "Category").map(intern);

        // Binds resolve their own bind-level activation modes into arena indices.
        let (default_binds, bind_errors) = Binds::from_node(node, activation_arena);

        // Action-level activation mode (fallback for binds that don’t specify a mode)
        let action_level_mode = ActivationMode::resolve(node, None, activation_arena);

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

    /// Prefer localized label; fall back to `action_name`.
    pub fn get_label(&self, translations: &HashMap<String, String>) -> String {
        let key = self.ui_label.as_deref().unwrap_or(&self.action_name);
        get_translation(key, translations).to_string()
    }

    /// Human summary of binds (keyboard | mouse). Returns `None` if both sides are empty.
    pub fn get_binds_label(&self) -> Option<String> {
        let binds = self.custom_binds.as_ref().unwrap_or(&self.default_binds);

        let kb = binds
            .keyboard
            .iter()
            .filter(|b| !b.is_unbound)
            .map(|b| b.to_string())
            .collect::<Vec<_>>();

        let mouse = binds
            .mouse
            .iter()
            .filter(|b| !b.is_unbound)
            .map(|b| b.to_string())
            .collect::<Vec<_>>();

        let mut parts = Vec::new();
        if !kb.is_empty() {
            parts.push(kb.join(", "));
        }
        if !mouse.is_empty() {
            parts.push(mouse.join(", "));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" | "))
        }
    }
}
