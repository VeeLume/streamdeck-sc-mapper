use std::{ collections::HashMap, sync::Arc };
use chrono::Local;
use crossbeam_channel::{ bounded, select, Receiver as CbReceiver };
use streamdeck_lib::prelude::*;

use crate::{
    bindings::{
        action_bindings::{ ActionBindings, ActionBindingsStore },
        constants::{ ACTION_MAP_UI_CATEGORIES, SKIP_ACTION_MAPS },
    },
    sc::topics::{
        ACTIONS_REQUEST,
        BINDINGS_PARSED,
        BINDINGS_REBUILD_AND_SAVE,
        INITIAL_INSTALL_SCAN_DONE,
        INSTALL_ACTIVE_CHANGED,
    },
    PLUGIN_ID,
};
use crate::sc::shared::{ appdata_dir, ActiveInstall, GameInstallType, InstallPaths, ResourceDir };

pub struct BindingsAdapter {
    /// used for AppData/bindings_<ty>.json and for controls/mappings/<PLUGIN_ID>.xml
    plugin_id: &'static str,
}

impl BindingsAdapter {
    pub fn new(plugin_id: &'static str) -> Self {
        Self { plugin_id }
    }
}

impl AdapterStatic for BindingsAdapter {
    const NAME: &'static str = "sc.bindings_adapter";
}

impl Adapter for BindingsAdapter {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn policy(&self) -> StartPolicy {
        StartPolicy::Eager
    }

    fn topics(&self) -> &'static [&'static str] {
        &[
            BINDINGS_REBUILD_AND_SAVE.name,
            ACTIONS_REQUEST.name,
            INITIAL_INSTALL_SCAN_DONE.name,
            INSTALL_ACTIVE_CHANGED.name,
        ]
    }

    fn start(
        &self,
        cx: &Context,
        bus: Arc<dyn Bus>,
        inbox: CbReceiver<Arc<ErasedTopic>>
    ) -> AdapterResult {
        let (stop_tx, stop_rx) = bounded::<()>(1);
        let logger = cx.log().clone();

        let installs = cx
            .try_ext::<InstallPaths>()
            .ok_or(AdapterError::Init("InstallPaths ext missing".to_string()))?
            .clone();
        let store = cx
            .try_ext::<ActionBindingsStore>()
            .ok_or(AdapterError::Init("ActionBindingsStore ext missing".to_string()))?
            .clone();
        let res_dir = cx
            .try_ext::<ResourceDir>()
            .ok_or(AdapterError::Init("ResourceDir ext missing".to_string()))?
            .clone();
        let active_install = cx
            .try_ext::<ActiveInstall>()
            .ok_or(AdapterError::Init("ActiveInstall ext missing".to_string()))?
            .clone();

        let plugin_id = self.plugin_id;

        let join = std::thread::spawn(move || {
            info!(logger, "BindingsAdapter started");

            // --- main loop --------------------------------------------------
            loop {
                select! {
                    recv(inbox) -> msg => match msg {
                        Ok(ev) => {
                            if let Some(m) = ev.downcast(BINDINGS_REBUILD_AND_SAVE) {
                                let game_path = match installs.get(m.ty) {
                                    Some(path) => path,
                                    None => {
                                        warn!(logger, "no install path for {:?}", m.ty);
                                        continue;
                                    }
                                };

                                debug!(logger, "BINDINGS_REBUILD_AND_SAVE for {:?}", m.ty);

                                let mut ab = match parse_xml(
                                    &game_path,
                                    &res_dir.get(),
                                    m.ty,
                                    m.with_custom,
                                    &logger
                                ) {
                                    Some(ab) => ab,
                                    None => {
                                        warn!(logger, "Failed to parse XML for {:?}", m.ty);
                                        continue;
                                    }
                                };

                                ab.generate_missing_binds(&logger);

                                save(
                                    &ab,
                                    &game_path,
                                    m.name.clone(),
                                    plugin_id,
                                    m.ty,
                                    &logger
                                );


                                // Store in ActionBindingsStore
                                debug!(logger, "Storing ActionBindings in store");
                                store.replace(ab);

                                bus.action_notify_topic_t(
                                    BINDINGS_PARSED,
                                    None,
                                    ()
                                );


                                continue;
                            }

                            if ev.downcast(INITIAL_INSTALL_SCAN_DONE).is_some() {
                                // Initial scan done, load bindings for the active install
                                let active = active_install.get();
                                debug!(logger, "INITIAL_INSTALL_SCAN_DONE for {:?}", active);

                                let ab = match load_from_json(active, &logger) {
                                    Ok(ab) => ab,
                                    Err(e) => {
                                        warn!(logger, "Failed to load bindings from JSON: {}", e);
                                        continue;
                                    }
                                };

                                // Store in ActionBindingsStore
                                debug!(logger, "Storing ActionBindings in store");
                                store.replace(ab);
                                continue;
                            }

                            if let Some(m) = ev.downcast(INSTALL_ACTIVE_CHANGED) {
                                // Clear and re-parse for the new active install
                                store.clear();
                                debug!(logger, "INSTALL_ACTIVE_CHANGED for {:?}", m.ty);

                                let mut ab = match load_from_json(m.ty, &logger) {
                                    Ok(ab) => ab,
                                    Err(e) => {
                                        warn!(logger, "Failed to load bindings from JSON: {}", e);
                                        ActionBindings::default()
                                    }
                                };

                                // If the ab is empty, try to parse XML
                                if ab.action_maps.is_empty() {
                                    debug!(logger, "No action maps found, trying XML for {:?}", m.ty);
                                    let game_path = match installs.get(m.ty) {
                                        Some(path) => path,
                                        None => {
                                            warn!(logger, "no install path for {:?}", m.ty);
                                            continue;
                                        }
                                    };

                                    if let Some(parsed_ab) = parse_xml(
                                        &game_path,
                                        &res_dir.get(),
                                        m.ty,
                                        true, // with_custom
                                        &logger
                                    ) {
                                        ab = parsed_ab;
                                    } else {
                                        warn!(logger, "Failed to parse XML for {:?}", m.ty);
                                        continue;
                                    }
                                }

                                // Store in ActionBindingsStore
                                debug!(logger, "Storing ActionBindings in store");
                                store.replace(ab);
                                continue;
                            }

                            // else: not for us
                        }

                        Err(e) => error!(logger, "recv: {}", e),
                    },

                    recv(stop_rx) -> _ => break,
                }
            }

            info!(logger, "BindingsAdapter stopped");
        });

        Ok(AdapterHandle::from_crossbeam(join, stop_tx))
    }
}

pub fn load_translations(
    path: std::path::PathBuf,
    logger: &Arc<dyn ActionLog>
) -> HashMap<String, String> {
    if !path.try_exists().unwrap_or(false) {
        warn!(logger, "no translations at {}", path.display());
        return HashMap::new();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            warn!(logger, "read {}: {}", path.display(), e);
            return HashMap::new();
        }
    };

    fn parse_line(line: &str) -> Option<(&str, &str)> {
        if let Some(i) = line.find(",P=") {
            let (k, v) = line.split_at(i);
            return Some((k.trim(), v.trim_start_matches(",P=").trim()));
        }
        if let Some(i) = line.find(',') {
            let (k, v) = line.split_at(i);
            return Some((k.trim(), v.trim_start_matches(',').trim()));
        }
        if let Some(i) = line.find('=') {
            let (k, v) = line.split_at(i);
            return Some((k.trim(), v.trim_start_matches('=').trim()));
        }
        None
    }

    let mut map = HashMap::new();
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = parse_line(t) {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

fn parse_xml(
    game_path: &std::path::PathBuf,
    resource_dir: &std::path::PathBuf,
    ty: GameInstallType,
    with_custom: bool,
    logger: &Arc<dyn ActionLog>
) -> Option<ActionBindings> {
    let default_profile = resource_dir.join("defaultProfile.xml");
    let custom_file = if with_custom {
        Some(
            game_path
                .join("user")
                .join("client")
                .join("0")
                .join("Profiles")
                .join("default")
                .join("actionmaps.xml")
        )
    } else {
        None
    };

    let mut ab = ActionBindings::default();
    let res = ab
        .load_default_profile(
            &default_profile,
            &SKIP_ACTION_MAPS,
            &ACTION_MAP_UI_CATEGORIES,
            logger
        )
        .ok();

    if res.is_some() {
        if let Some(cf) = custom_file {
            if cf.try_exists().unwrap_or(false) {
                if let Err(e) = ab.apply_custom_profile(&cf, logger) {
                    warn!(logger, "apply_custom_profile({:?}): {}", ty, e);
                }
            } else {
                debug!(logger, "no custom file at {}", cf.display());
            }
        }
        ab.activation.rebuild_indexes(); // <- important
        Some(ab)
    } else {
        None
    }
}

fn load_from_json(
    ty: GameInstallType,
    logger: &Arc<dyn ActionLog>
) -> Result<ActionBindings, String> {
    let base = appdata_dir(PLUGIN_ID).map_err(|e|
        format!("Failed to get AppData directory: {}", e)
    )?;
    let file = base.join(format!("bindings_{}.json", ty.name()));
    if !file.try_exists().unwrap_or(false) {
        return Err(format!("No bindings file found at {}", file.display()));
    }
    let content = std::fs
        ::read_to_string(&file)
        .map_err(|e| format!("Failed to read bindings file: {}", e))?;
    let mut ab = ActionBindings::default();
    ab.from_json(&content, logger)?;

    ab.activation.rebuild_indexes(); // <- important
    info!(
        logger,
        "Loaded {} action maps with {} activation modes for {:?}",
        ab.action_maps.len(),
        ab.activation.len(),
        ty
    );
    Ok(ab)
}

fn save(
    ab: &ActionBindings,
    game_path: &std::path::PathBuf,
    profile_name: Option<String>,
    plugin_id: &str,
    ty: GameInstallType,
    logger: &Arc<dyn ActionLog>
) {
    // write profile.xml …
    let profile_dir = game_path
        .join("user")
        .join("client")
        .join("0")
        .join("controls")
        .join("mappings");

    let _ = std::fs::create_dir_all(&profile_dir);
    let profile_name = profile_name.unwrap_or_else(|| {
        format!("{}-{}-{}", "SC-Mapper", ty.name(), Local::now().format("%Y%m%d-%H:%M"))
    });

    let profile_path = profile_dir.join(format!("{}.xml", PLUGIN_ID));
    if let Err(e) = ab.generate_mapping_xml(profile_path.clone(), None, &profile_name) {
        warn!(logger, "generate_mapping_xml: {}", e);
    } else {
        info!(logger, "wrote profile {}", profile_path.display());
    }

    if
        let Err(e) = ab.to_json().and_then(|json| {
            if let Ok(base) = appdata_dir(plugin_id) {
                let f = base.join(format!("bindings_{}.json", ty.name()));
                Ok(std::fs::write(&f, json))
            } else {
                Err("Failed to get AppData directory".to_string())
            }
        })
    {
        warn!(logger, "Failed to write JSON: {}", e);
    } else {
        info!(logger, "Wrote bindings_{}.json", ty.name());
    }
}
