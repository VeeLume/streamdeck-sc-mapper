use std::{ collections::{ HashMap, HashSet }, sync::Arc };

use indexmap::IndexMap;
use streamdeck_lib::prelude::*;

use crate::bindings::{
    action_map::ActionMap,
    activation_mode::ActivationArena,
    bind::{ Bind, BindMain },
    binds::Binds,
    constants::{
        CANDIDATE_KEYS,
        CANDIDATE_MODIFIERS,
        CATEGORY_GROUPS,
        DEFAULT_CATEGORY,
        DENY_COMBOS,
        DISSALOWED_MODIFIERS_PER_CATEGORY,
    },
};

/// Generates missing binds using available keys/modifiers and category rules.
pub struct BindGenerator {
    pub available_keys: HashSet<Key>,
    pub available_modifiers: HashSet<Key>,
    pub banned_binds: HashSet<Bind>,
    pub group_map: HashMap<String, HashSet<String>>,
    pub disallowed_modifiers: HashMap<String, HashSet<Key>>,
    /// Arena index of the "press" activation mode (if present)
    pub press_idx: Option<usize>,
    pub logger: Arc<dyn ActionLog>,

    /// Tracks used binds per group to avoid collisions.
    pub used_binds_by_group: HashMap<String, HashSet<Bind>>,
}

impl BindGenerator {
    /// Construct with explicit pools and an arena to resolve "press".
    pub fn new(
        modes: &ActivationArena,
        available_keys: HashSet<Key>,
        available_modifiers: HashSet<Key>,
        banned_binds: HashSet<Bind>,
        group_map: HashMap<String, HashSet<String>>,
        disallowed_modifiers: HashMap<String, HashSet<Key>>,
        logger: Arc<dyn ActionLog>
    ) -> Self {
        let press_idx = modes
            .iter()
            .find(|(_, am)| am.name.as_deref() == Some("press"))
            .map(|(idx, _)| idx);

        Self {
            available_keys,
            available_modifiers,
            banned_binds,
            group_map,
            disallowed_modifiers,
            press_idx,
            logger,
            used_binds_by_group: HashMap::new(),
        }
    }

    /// Sensible defaults: use constants and find "press" in the arena.
    pub fn default(logger: Arc<dyn ActionLog>, modes: &ActivationArena) -> Self {
        let group_map = CATEGORY_GROUPS.iter()
            .map(|(k, v)| (
                k.to_string(),
                v
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            ))
            .collect::<HashMap<_, HashSet<_>>>();

        let disallowed_modifiers = DISSALOWED_MODIFIERS_PER_CATEGORY.iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    v
                        .iter()
                        .filter_map(|s| Key::parse(s))
                        .collect::<HashSet<Key>>(),
                )
            })
            .collect::<HashMap<_, _>>();

        Self::new(
            modes,
            CANDIDATE_KEYS.clone(),
            CANDIDATE_MODIFIERS.clone(),
            DENY_COMBOS.clone(),
            group_map,
            disallowed_modifiers,
            logger
        )
    }

    /// Seed `used_binds_by_group` with existing binds.
    pub fn register_existing_binds(&mut self, action_maps: &IndexMap<Arc<str>, ActionMap>) {
        for action_map in action_maps.values() {
            let category = action_map.ui_category
                .as_deref()
                .unwrap_or(DEFAULT_CATEGORY)
                .to_string();

            // category → groups (may map to multiple)
            let groups = self.group_map
                .get(&category)
                .cloned()
                .unwrap_or_else(|| HashSet::from([category.clone()]));

            for binding in action_map.actions.values() {
                let existing_iter = binding.default_binds
                    .all_binds()
                    .chain(
                        binding.custom_binds
                            .as_ref()
                            .map_or(
                                Box::new(std::iter::empty()) as Box<dyn Iterator<Item = Bind>>,
                                |cb| { Box::new(cb.all_binds()) }
                            )
                    );

                let all_vec: Vec<Bind> = existing_iter.collect();
                for g in &groups {
                    self.used_binds_by_group
                        .entry(g.clone())
                        .or_default()
                        .extend(all_vec.iter().cloned());
                }
            }
        }
    }

    /// Suggest the next unused bind for a category (respecting bans & group usage).
    pub fn next_available_bind(&mut self, category: &str) -> Option<Bind> {
        let groups = self.group_map
            .get(category)
            .cloned()
            .unwrap_or_else(|| HashSet::from([category.to_string()]));

        // Compute allowed modifier pool for this category.
        let disallowed_mods = self.resolve_disallowed_modifiers(category);
        let allowed_mods = self.available_modifiers
            .difference(&disallowed_mods)
            .cloned()
            .collect::<HashSet<_>>();

        for key in &self.available_keys {
            for mod_combo in Self::generate_modifier_combos(&allowed_mods) {
                let candidate = Bind::generated(BindMain::Key(*key), mod_combo, self.press_idx);

                if self.banned_binds.contains(&candidate) {
                    continue;
                }

                // Used in any group?
                let used = groups
                    .iter()
                    .any(|g| {
                        self.used_binds_by_group.get(g).map_or(false, |s| s.contains(&candidate))
                    });
                if used {
                    continue;
                }

                // Reserve in all groups and return.
                for g in &groups {
                    self.used_binds_by_group
                        .entry(g.clone())
                        .or_default()
                        .insert(candidate.clone());
                }
                return Some(candidate);
            }
        }
        None
    }

    fn resolve_disallowed_modifiers(&self, category: &str) -> HashSet<Key> {
        self.group_map
            .get(category)
            .into_iter()
            .flatten()
            .flat_map(|c| self.disallowed_modifiers.get(c).into_iter().flatten().cloned())
            .collect()
    }

    fn generate_modifier_combos(mods: &HashSet<Key>) -> Vec<HashSet<Key>> {
        let v: Vec<_> = mods.iter().copied().collect();
        let mut out = Vec::new();

        out.push(HashSet::new()); // empty (no modifiers)

        for i in 0..v.len() {
            let one = HashSet::from([v[i]]);
            out.push(one.clone());

            for j in i + 1..v.len() {
                let mut two = one.clone();
                two.insert(v[j]);
                out.push(two);
            }
        }
        out
    }

    /// Fill gaps across all actions (custom > default).
    pub fn generate_missing_binds(&mut self, action_maps: &mut IndexMap<Arc<str>, ActionMap>) {
        self.register_existing_binds(action_maps);

        for (map_name, action_map) in action_maps.iter_mut() {
            let category = action_map.ui_category
                .as_deref()
                .unwrap_or(DEFAULT_CATEGORY)
                .to_string();

            for binding in action_map.actions.values_mut() {
                let has_default = binding.default_binds.has_active_binds();
                let has_custom = binding.custom_binds
                    .as_ref()
                    .map_or(false, |b| b.has_active_binds());

                if has_default || has_custom {
                    continue;
                }

                if let Some(candidate) = self.next_available_bind(&category) {
                    binding.custom_binds = Some(Binds {
                        keyboard: vec![candidate.clone()],
                        mouse: vec![],
                    });

                    let _ = self.logger.log(
                        &format!(
                            "✅ Generated bind for {}.{}: {}",
                            map_name,
                            binding.action_name,
                            candidate
                        )
                    );
                } else {
                    let _ = self.logger.log(
                        &format!("⚠️ No available bind for {}.{}", map_name, binding.action_name)
                    );
                }
            }
        }

        let _ = self.logger.log("[generate_missing_binds] Done generating binds");
    }
}
