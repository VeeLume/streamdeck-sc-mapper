use std::{ collections::{ HashMap, HashSet }, sync::Arc };

use indexmap::IndexMap;

use crate::{
    action_binds::{
        action_map::ActionMap,
        activation_mode::ActivationMode,
        bind::Bind,
        binds::Binds,
        constants::{
            CANDIDATE_KEYS,
            CANDIDATE_MODIFIERS,
            CATEGORY_GROUPS,
            DEFAULT_CATEGORY,
            DENY_COMBOS,
            DISSALOWED_MODIFIERS_PER_CATEGORY,
        },
    },
    logger::ActionLog,
};

pub struct BindGenerator {
    pub available_keys: HashSet<String>,
    pub available_modifiers: HashSet<String>,
    pub banned_binds: HashSet<Bind>,
    pub group_map: HashMap<String, HashSet<String>>,
    pub disallowed_modifiers: HashMap<String, HashSet<String>>,
    pub activation_mode_press: Option<ActivationMode>,
    pub logger: Arc<dyn ActionLog>,

    pub used_binds_by_group: HashMap<String, HashSet<Bind>>,
}

impl BindGenerator {
    pub fn new(
        activation_modes: Vec<ActivationMode>,
        available_keys: HashSet<String>,
        available_modifiers: HashSet<String>,
        banned_binds: HashSet<Bind>,
        group_map: HashMap<String, HashSet<String>>,
        disallowed_modifiers: HashMap<String, HashSet<String>>,
        logger: Arc<dyn ActionLog>
    ) -> Self {
        let activation_mode_press = activation_modes
            .into_iter()
            .find(|am| am.name.as_deref() == Some("press"));

        BindGenerator {
            available_keys,
            available_modifiers,
            banned_binds,
            group_map,
            disallowed_modifiers,
            activation_mode_press,
            logger,
            used_binds_by_group: HashMap::new(),
        }
    }

    pub fn default(logger: Arc<dyn ActionLog>, activation_modes: Vec<ActivationMode>) -> Self {
        Self::new(
            activation_modes,
            CANDIDATE_KEYS.iter()
                .map(|s| s.to_string())
                .collect(),
            CANDIDATE_MODIFIERS.iter()
                .map(|s| s.to_string())
                .collect(),
            DENY_COMBOS.iter().cloned().collect(),
            CATEGORY_GROUPS.iter()
                .map(|(k, v)| (
                    k.to_string(),
                    v
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                ))
                .collect(),
            DISSALOWED_MODIFIERS_PER_CATEGORY.iter()
                .map(|(k, v)| (
                    k.to_string(),
                    v
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                ))
                .collect(),
            logger
        )
    }

    pub fn register_existing_binds(&mut self, action_maps: &IndexMap<String, ActionMap>) {
        for action_map in action_maps.values() {
            let category = action_map.ui_category
                .as_deref()
                .unwrap_or(DEFAULT_CATEGORY)
                .to_string();

            let group = self.group_map
                .get(&category)
                .cloned()
                .unwrap_or_else(|| HashSet::from([category.clone()]));

            for binding in action_map.actions.values() {
                let all_binds = binding.default_binds
                    .all_binds()
                    .chain(
                        binding.custom_binds
                            .as_ref()
                            .map_or(
                                Box::new(std::iter::empty()) as Box<dyn Iterator<Item = Bind>>,
                                |cb| { Box::new(cb.all_binds()) }
                            )
                    );

                let all_binds_vec: Vec<Bind> = all_binds.collect();
                for g in &group {
                    self.used_binds_by_group
                        .entry(g.clone())
                        .or_default()
                        .extend(all_binds_vec.iter().cloned());
                }
            }
        }
    }

    pub fn next_available_bind(&mut self, category: &str) -> Option<Bind> {
        let group = self.group_map
            .get(category)
            .cloned()
            .unwrap_or_else(|| HashSet::from([category.to_string()]));

        let disallowed_mods = self.resolve_disallowed_modifiers(category);
        let allowed_mods = self.available_modifiers
            .difference(&disallowed_mods)
            .cloned()
            .collect::<HashSet<_>>();

        for key in &self.available_keys {
            for mod_combo in Self::generate_modifier_combos(&allowed_mods) {
                let candidate = Bind {
                    mainkey: key.clone(),
                    modifiers: mod_combo,
                    activation_mode: self.activation_mode_press.clone(),
                    is_unbound: false,
                };

                if self.banned_binds.contains(&candidate) {
                    continue;
                }

                let is_used = group
                    .iter()
                    .any(|g| {
                        self.used_binds_by_group.get(g).map_or(false, |s| s.contains(&candidate))
                    });

                if is_used {
                    continue;
                }

                for g in &group {
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

    fn resolve_disallowed_modifiers(&self, category: &str) -> HashSet<String> {
        self.group_map
            .get(category)
            .into_iter()
            .flatten()
            .flat_map(|cat| { self.disallowed_modifiers.get(cat).into_iter().flatten().cloned() })
            .collect()
    }

    fn generate_modifier_combos(mods: &HashSet<String>) -> Vec<HashSet<String>> {
        let mods_vec: Vec<_> = mods.iter().cloned().collect();
        let mut combos = Vec::new();

        combos.push(HashSet::new()); // No modifiers

        for i in 0..mods_vec.len() {
            let one = HashSet::from([mods_vec[i].clone()]);
            combos.push(one.clone());

            for j in i + 1..mods_vec.len() {
                let mut two = one.clone();
                two.insert(mods_vec[j].clone());
                combos.push(two);
            }
        }

        combos
    }

    pub fn generate_missing_binds(
        &mut self,
        action_maps: &mut IndexMap<String, ActionMap>,
        logger: Arc<dyn ActionLog>
    ) {
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

                    let _ = logger.log(
                        &format!(
                            "✅ Generated bind for {}.{}: {:?}",
                            map_name,
                            binding.action_name,
                            candidate
                        )
                    );
                } else {
                    let _ = logger.log(
                        &format!("⚠️ No available bind for {}.{}", map_name, binding.action_name)
                    );
                }
            }
        }

        let _ = logger.log("[generate_missing_binds] Done generating binds");
    }
}
