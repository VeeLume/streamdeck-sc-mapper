use indexmap::IndexMap;
use roxmltree::Document;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path, sync::Arc};

use crate::bindings::{
    action_map::ActionMap,
    activation_mode::{ActivationArena, ActivationMode},
    bind::Bind,
    binds::Binds,
    binds_generator::BindGenerator,
};
use crate::core_log::CoreLog;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionBindings {
    pub action_maps: IndexMap<Arc<str>, ActionMap>,
    pub activation: ActivationArena,
}

impl ActionBindings {
    /// Load defaults from the game "defaultProfile.xml" and replace current data.
    pub fn load_default_profile<P: AsRef<Path>>(
        &mut self,
        path: P,
        skip_actionmaps: &std::collections::HashSet<String>,
        actionmap_ui_categories: &HashMap<String, String>,
        logger: &Arc<dyn CoreLog>,
    ) -> Result<(), String> {
        let content =
            fs::read_to_string(&path).map_err(|e| format!("read default profile: {e}"))?;
        let doc = Document::parse(&content).map_err(|e| format!("parse default XML: {e}"))?;

        let mut ab = ActionBindings::default();

        // ActivationMode nodes (dedupe by semantics+name)
        for node in doc
            .descendants()
            .filter(|n| n.has_tag_name("ActivationMode"))
        {
            let mode = ActivationMode::from_node(node, true);
            let _ = ActivationMode::insert_or_get(&mut ab.activation, mode);
        }

        for node in doc.descendants().filter(|n| n.has_tag_name("actionmap")) {
            let Some(name) = node.attribute("name") else {
                continue;
            };
            if skip_actionmaps.contains(name) {
                continue;
            }

            match ActionMap::from_node(node, &mut ab.activation, actionmap_ui_categories) {
                Ok((amap, parse_errors)) => {
                    ab.action_maps.insert(amap.name.clone(), amap);
                    for e in parse_errors {
                        logger.warn(&format!(
                            "[load_default_profile] parse error in {name}: {e:?}"
                        ));
                    }
                }
                Err(e) => {
                    logger.warn(&format!(
                        "[load_default_profile] failed to parse {name}: {e:?}"
                    ));
                }
            }
        }

        let total_actions: usize = ab.action_maps.values().map(|m| m.actions.len()).sum();
        logger.info(&format!(
            "[load_default_profile] Loaded {} actions in {} maps; {} activation modes",
            total_actions,
            ab.action_maps.len(),
            ab.activation.len()
        ));

        ab.activation.rebuild_indexes();

        self.action_maps = ab.action_maps;
        self.activation = ab.activation;

        Ok(())
    }

    /// Overlay custom rebinds (user profile) onto the current graph.
    pub fn apply_custom_profile<P: AsRef<Path>>(
        &mut self,
        path: P,
        logger: &Arc<dyn CoreLog>,
    ) -> Result<(), String> {
        let content = fs::read_to_string(&path).map_err(|e| format!("read custom profile: {e}"))?;
        let doc = Document::parse(&content).map_err(|e| format!("parse custom XML: {e}"))?;

        for am_node in doc.descendants().filter(|n| n.has_tag_name("actionmap")) {
            let Some(am_name) = am_node.attribute("name") else {
                continue;
            };

            for act_node in am_node.children().filter(|n| n.has_tag_name("action")) {
                let Some(act_name) = act_node.attribute("name") else {
                    continue;
                };

                let mut binds = Binds::new();

                for rebind in act_node.children().filter(|n| n.has_tag_name("rebind")) {
                    let input = rebind.attribute("input").unwrap_or("").trim();
                    // Expect prefixes like kb1_, mo1_...
                    let (prefix, key_str) = match input.get(..3).zip(input.get(3..)) {
                        Some((p, rest)) => (p, rest.trim()),
                        None => {
                            logger.warn(&format!(
                                "[apply_custom_profile] bad input '{input}' on {am_name}.{act_name}"
                            ));
                            continue;
                        }
                    };

                    let am_ix = rebind
                        .attribute("activationMode")
                        .and_then(|name| self.activation.find_by_name(name));

                    match Bind::from_string(key_str, am_ix) {
                        Ok(b) => match prefix {
                            "kb1" => binds.keyboard.push(b),
                            "mo1" => binds.mouse.push(b),
                            _ => logger.debug(&format!(
                                "[apply_custom_profile] ignoring device '{prefix}' on {am_name}.{act_name}"
                            )),
                        },
                        Err(e) => logger.warn(&format!(
                            "[apply_custom_profile] parse bind {am_name}.{act_name}: {e:?}"
                        )),
                    }
                }

                if let Some(amap) = self.action_maps.get_mut(am_name) {
                    if let Some(abind) = amap.actions.get_mut(act_name) {
                        abind.custom_binds = Some(binds);
                    }
                }
            }
        }

        logger.info("[apply_custom_profile] Finished applying custom rebinds");
        Ok(())
    }

    /// Generate missing binds via the generator (fills gaps in-place).
    pub fn generate_missing_binds(&mut self, logger: &Arc<dyn CoreLog>) {
        let mut bind_gen = BindGenerator::with_logger(&self.activation, logger);
        bind_gen.generate_missing_binds(&mut self.action_maps);
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self).map_err(|e| format!("serialize ActionBindings: {e}"))
    }

    pub fn from_json(content: &str, logger: &Arc<dyn CoreLog>) -> Result<Self, String> {
        let mut data: ActionBindings = serde_json::from_str(content)
            .map_err(|e| format!("deserialize ActionBindings: {e}"))?;
        data.activation.rebuild_indexes(); // important for lookups
        logger.info(&format!(
            "✅ Loaded {} action maps with {} activation modes",
            data.action_maps.len(),
            data.activation.len()
        ));
        Ok(data)
    }
}
