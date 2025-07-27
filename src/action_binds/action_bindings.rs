use std::{ collections::{ HashMap, HashSet }, fs, path::Path, sync::Arc };

use indexmap::IndexMap;
use roxmltree::Document;
use serde::{ Deserialize, Serialize };

use crate::{
    action_binds::{
        action_map::ActionMap,
        activation_mode::ActivationMode,
        bind::Bind,
        bind_generator::BindGenerator,
        binds::Binds,
    },
    logger::ActionLog,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBindingsData {
    pub action_maps: IndexMap<String, ActionMap>,
    pub activation_modes: Vec<ActivationMode>,
}

pub struct ActionBindings {
    pub action_maps: IndexMap<String, ActionMap>,
    pub activation_modes: Vec<ActivationMode>,
    pub logger: Arc<dyn ActionLog>,
}

impl ActionBindings {
    pub fn get_binding_by_id(
        &self,
        id: &str
    ) -> Option<&crate::action_binds::action_binding::ActionBinding> {
        let mut parts = id.splitn(2, '.');
        let action_map_name = parts.next()?;
        let action_name = parts.next()?;

        self.action_maps.get(action_map_name).and_then(|map| map.actions.get(action_name))
    }

    pub fn new(logger: Arc<dyn ActionLog>) -> Self {
        Self {
            action_maps: IndexMap::new(),
            activation_modes: Vec::new(),
            logger,
        }
    }

    pub fn load_json(&mut self, content: &String) -> Result<(), String> {
        let data: ActionBindingsData = serde_json
            ::from_str(&content)
            .map_err(|e| format!("Failed to deserialize ActionBindingsData: {e}"))?;

        self.action_maps = data.action_maps;
        self.activation_modes = data.activation_modes;

        self.logger.log(
            &format!(
                "✅ Loaded {} action maps with {} activation modes",
                self.action_maps.len(),
                self.activation_modes.len()
            )
        );

        Ok(())
    }

    pub fn save_json(&self) -> Result<String, String> {
        let data = ActionBindingsData {
            action_maps: self.action_maps.clone(),
            activation_modes: self.activation_modes.clone(),
        };

        let json = serde_json
            ::to_string_pretty(&data)
            .map_err(|e| format!("Failed to serialize ActionBindingsData: {e}"))?;

        Ok(json)
    }

    pub fn load_default_profile<P: AsRef<Path>>(
        &mut self,
        path: P,
        skip_actionmaps: &HashSet<String>,
        actionmap_ui_categories: &HashMap<String, String>
    ) -> Result<(), String> {
        let content = fs
            ::read_to_string(&path)
            .map_err(|e| format!("Failed to read default profile: {e}"))?;

        let doc = Document::parse(&content).map_err(|e| format!("Failed to parse XML: {e}"))?;

        // Parse <ActivationMode> nodes
        for node in doc.descendants().filter(|n| n.has_tag_name("ActivationMode")) {
            let mode = ActivationMode::from_node(node, true);
            self.activation_modes.push(mode);
        }

        // Parse <actionmap> nodes
        for node in doc.descendants().filter(|n| n.has_tag_name("actionmap")) {
            let name = match node.attribute("name") {
                Some(n) => n,
                None => {
                    self.logger.log("[load_default_profile] Skipped actionmap with missing name");
                    continue;
                }
            };

            if skip_actionmaps.contains(name) {
                self.logger.log(&format!("[load_default_profile] Skipped actionmap: {name}"));
                continue;
            }

            match ActionMap::from_node(node, &self.activation_modes, actionmap_ui_categories) {
                Ok((action_map, parse_errors)) => {
                    self.action_maps.insert(action_map.name.clone(), action_map);

                    for error in parse_errors {
                        self.logger.log(
                            &format!("[load_default_profile] Error in action {}: {error:?}", name)
                        );
                    }
                }
                Err(e) => {
                    self.logger.log(
                        &format!("[load_default_profile] Failed to parse actionmap {name}: {e:?}")
                    );
                }
            }
        }

        self.logger.log(
            &format!(
                "[load_default_profile] Loaded {} actions in {} action maps with {} activation modes",
                self.action_maps
                    .values()
                    .map(|map| map.actions.len())
                    .sum::<usize>(),
                self.action_maps.len(),
                self.activation_modes.len()
            )
        );

        Ok(())
    }

    pub fn apply_custom_profile<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let content = fs
            ::read_to_string(&path)
            .map_err(|e| format!("Failed to read custom profile: {e}"))?;

        let doc = Document::parse(&content).map_err(|e|
            format!("Failed to parse custom XML: {e}")
        )?;

        for actionmap_node in doc.descendants().filter(|n| n.has_tag_name("actionmap")) {
            let action_map_name = match actionmap_node.attribute("name") {
                Some(name) => name,
                None => {
                    continue;
                }
            };

            for action_node in actionmap_node.children().filter(|n| n.has_tag_name("action")) {
                let action_name = match action_node.attribute("name") {
                    Some(name) => name,
                    None => {
                        continue;
                    }
                };

                let mut binds = Binds::new();

                for rebind_node in action_node.children().filter(|n| n.has_tag_name("rebind")) {
                    let input = rebind_node.attribute("input").unwrap_or("").trim();

                    let (device_type, key_str) = match input.get(..3).zip(input.get(3..)) {
                        Some((prefix, rest)) => (prefix, rest.trim()),
                        None => {
                            self.logger.log(
                                &format!(
                                    "[apply_custom_profile] Invalid input: {input:?} on action {action_map_name}.{action_name}"
                                )
                            );
                            continue;
                        }
                    };

                    let activation_mode = rebind_node.attribute("activationMode").and_then(|name|
                        self.activation_modes
                            .iter()
                            .find(|am| am.name.as_deref() == Some(name))
                            .cloned()
                    );

                    match Bind::from_string(key_str, activation_mode) {
                        Ok(bind) =>
                            match device_type {
                                "kb1" => binds.keyboard.push(bind),
                                "mo1" => binds.mouse.push(bind),
                                _ => {
                                    self.logger.log(
                                        &format!(
                                            "[apply_custom_profile] Ignoring device prefix: {device_type} on action {action_map_name}.{action_name}"
                                        )
                                    );
                                }
                            }
                        Err(e) => {
                            self.logger.log(
                                &format!(
                                    "[apply_custom_profile] Failed to parse bind for {action_map_name}.{action_name}: {e:?}"
                                )
                            );
                        }
                    }
                }

                // Apply custom binds regardless of whether any are active
                if let Some(action_map) = self.action_maps.get_mut(action_map_name) {
                    if let Some(binding) = action_map.actions.get_mut(action_name) {
                        binding.custom_binds = Some(binds.clone());

                        if binds.has_active_binds() {
                            self.logger.log(
                                &format!(
                                    "✅ Updated custom binds for {}.{}: {} | {}",
                                    action_map_name,
                                    action_name,
                                    binds.keyboard
                                        .iter()
                                        .map(|b| b.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                    binds.mouse
                                        .iter()
                                        .map(|b| b.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )
                            );
                        } else {
                            self.logger.log(
                                &format!(
                                    "🛑 Unbound all inputs for {}.{}",
                                    action_map_name,
                                    action_name
                                )
                            );
                        }
                    }
                }
            }
        }

        self.logger.log("[apply_custom_profile] Finished applying custom rebinds");

        Ok(())
    }

    pub fn generate_missing_binds(&mut self) {
        let mut generator = BindGenerator::default(
            self.logger.clone(),
            self.activation_modes.clone()
        );

        generator.generate_missing_binds(&mut self.action_maps);
    }
}
