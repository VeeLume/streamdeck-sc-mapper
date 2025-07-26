use roxmltree::Node;
use serde::{ Deserialize, Serialize };
use crate::action_binds::{
    activation_mode::ActivationMode,
    bind::BindParseError,
    binds::Binds,
    get_translation,
};
use std::collections::HashMap;

#[derive(Debug)]
pub enum ActionBindingParseError {
    MissingName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBinding {
    pub action_id: String,
    pub action_name: String,
    pub ui_label: Option<String>,
    pub ui_description: Option<String>,
    pub category: Option<String>,
    pub default_binds: Binds,
    pub custom_binds: Option<Binds>,
    pub activation_mode: Option<ActivationMode>,
}

impl ActionBinding {
    pub fn from_node(
        node: Node,
        action_map_name: &str,
        activation_modes: &[ActivationMode]
    ) -> Result<(Self, Vec<BindParseError>), ActionBindingParseError> {
        let name = node.attribute("name").ok_or(ActionBindingParseError::MissingName)?.to_string();
        let action_id = format!("{}.{}", action_map_name, name);

        let ui_label = node
            .attribute("UILabel")
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string);
        let ui_description = node
            .attribute("UIDescription")
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string);
        let category = node
            .attribute("Category")
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string);

        let (default_binds, bind_errors) = Binds::from_node(node, activation_modes);

        let activation_mode = ActivationMode::resolve(node, None, activation_modes);

        Ok((
            ActionBinding {
                action_id,
                action_name: name,
                ui_label,
                ui_description,
                category,
                default_binds,
                custom_binds: None,
                activation_mode,
            },
            bind_errors,
        ))
    }

    pub fn get_label(&self, translations: &HashMap<String, String>) -> String {
        let key = self.ui_label.as_deref().unwrap_or(&self.action_name);
        get_translation(key, translations).to_string()
    }

    pub fn get_binds_label(&self) -> Option<String> {
        let binds = self.custom_binds.as_ref().unwrap_or(&self.default_binds);
        let keys: Vec<_> = binds.keyboard
            .iter()
            .map(|b| b.to_string())
            .collect();
        if keys.is_empty() {
            None
        } else {
            Some(keys.join(", "))
        }
    }
}
