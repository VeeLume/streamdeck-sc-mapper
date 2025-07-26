use roxmltree::Node;
use serde::{ Deserialize, Serialize };
use indexmap::IndexMap; // optional, but nice to preserve insertion order
use std::{ collections::HashMap, ops::Range };

use crate::action_binds::{
    action_binding::{ ActionBinding, ActionBindingParseError },
    activation_mode::ActivationMode,
    bind::BindParseError,
    get_translation,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMap {
    pub name: String,
    pub version: u32,
    pub ui_label: Option<String>,
    pub ui_category: Option<String>,
    pub actions: IndexMap<String, ActionBinding>, // key = action_name
}

#[derive(Debug)]
pub enum ActionMapParseError {
    MissingName,
    MissingVersion,
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
        let key = self.ui_label.as_deref().or(self.ui_category.as_deref()).unwrap_or(&self.name);
        get_translation(key, translations).to_string()
    }

    pub fn from_node(
        node: Node,
        activation_modes: &[ActivationMode],
        actionmap_ui_categories: &HashMap<String, String>
    ) -> Result<(Self, Vec<ActionParseError>), ActionMapParseError> {
        let name = node.attribute("name").ok_or(ActionMapParseError::MissingName)?.to_string();

        let version = node
            .attribute("version")
            .ok_or(ActionMapParseError::MissingVersion)
            .unwrap_or("1")
            .parse::<u32>()
            .map_err(|_| ActionMapParseError::MissingVersion)
            .unwrap_or(1); // Default to version 1 if parsing fails

        let ui_label = node
            .attribute("UILabel")
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string);
        let ui_category = node
            .attribute("UICategory")
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
            .or_else(|| actionmap_ui_categories.get(&name).cloned());

        let mut actions = IndexMap::new();
        let mut errors = Vec::new();

        for action_node in node.children().filter(|n| n.is_element() && n.has_tag_name("action")) {
            match ActionBinding::from_node(action_node, &name, activation_modes) {
                Ok((binding, bind_errors)) => {
                    let action_name = binding.action_name.clone();
                    actions.insert(action_name.clone(), binding);

                    errors.extend(
                        bind_errors.into_iter().map(|e| ActionParseError::BindError {
                            action_name: action_name.clone(),
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
                name,
                version,
                ui_label,
                ui_category,
                actions,
            },
            errors,
        ))
    }
}
