use roxmltree::Node;
use serde::{ Deserialize, Serialize };
use indexmap::IndexMap;
use std::{ collections::HashMap, ops::Range, sync::Arc };

use crate::bindings::{
    action_binding::{ ActionBinding, ActionBindingParseError },
    activation_mode::ActivationArena,
    bind::BindParseError,
    helpers::get_translation,
    str_intern::intern,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMap {
    pub name: Arc<str>,
    pub version: u32,
    // keep as String so we can just as_deref() cleanly in get_label
    pub ui_label: Option<String>,
    pub ui_category: Option<String>,
    // key by action_name (Arc<str>) to match ActionBinding.action_name type
    pub actions: IndexMap<Arc<str>, ActionBinding>,
}

#[derive(Debug)]
pub enum ActionMapParseError {
    MissingName,
}

#[derive(Debug)]
pub enum ActionParseError {
    ActionBindingError {
        action_name: Option<String>,
        range: Range<usize>,
        error: ActionBindingParseError,
    },
    BindError {
        action_name: String,
        bind_error: BindParseError,
    },
}

impl ActionMap {
    pub fn get_label(&self, translations: &HashMap<String, String>) -> String {
        // Prefer UILabel, then UICategory, fall back to map name
        let key = self.ui_label.as_deref().or(self.ui_category.as_deref()).unwrap_or(&self.name);
        get_translation(key, translations).to_string()
    }

    pub fn from_node(
        node: Node,
        activation_modes: &mut ActivationArena,
        actionmap_ui_categories: &HashMap<String, String>
    ) -> Result<(Self, Vec<ActionParseError>), ActionMapParseError> {
        // --- name & version ---
        let name_str = node.attribute("name").ok_or(ActionMapParseError::MissingName)?.to_string();

        let version = node
            .attribute("version")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1);

        // --- UI labels/categories (optional) ---
        let ui_label = node
            .attribute("UILabel")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let ui_category = node
            .attribute("UICategory")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            // fallback from your static map by actionmap name
            .or_else(|| actionmap_ui_categories.get(&name_str).cloned());

        // --- actions ---
        let mut actions: IndexMap<Arc<str>, ActionBinding> = IndexMap::new();
        let mut errors: Vec<ActionParseError> = Vec::new();

        for action_node in node.children().filter(|n| n.is_element() && n.has_tag_name("action")) {
            match ActionBinding::from_node(action_node, &name_str, activation_modes) {
                Ok((binding, bind_errors)) => {
                    let action_name = binding.action_name.clone(); // Arc<str>
                    actions.insert(action_name.clone(), binding);
                    errors.extend(
                        bind_errors.into_iter().map(|e| ActionParseError::BindError {
                            action_name: action_name.to_string(),
                            bind_error: e,
                        })
                    );
                }
                Err(e) => {
                    errors.push(ActionParseError::ActionBindingError {
                        action_name: action_node.attribute("name").map(str::to_string),
                        range: action_node.range(),
                        error: e,
                    });
                }
            }
        }

        Ok((
            ActionMap {
                name: intern(name_str),
                version,
                ui_label,
                ui_category,
                actions,
            },
            errors,
        ))
    }
}
