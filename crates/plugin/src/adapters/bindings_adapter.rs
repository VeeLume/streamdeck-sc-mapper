use crossbeam_channel::{Receiver as CbReceiver, bounded, select};
use std::sync::Arc;
use streamdeck_lib::prelude::*;
use streamdeck_sc_core::{
    CoreLog,
    prelude::ActionBindings,
    sc::profiles::{
        load_bindings_from_appdata, parse_bindings_from_install, save_bindings_profile_and_cache,
    },
};

use crate::{
    state::{
        action_bindings_store::ActionBindingsStore, active_install_store::ActiveInstall,
        install_paths_store::InstallPaths, resource_dir_store::ResourceDir,
    },
    topics::{
        ACTIONS_REQUEST, BINDINGS_PARSED, BINDINGS_REBUILD_AND_SAVE, INITIAL_INSTALL_SCAN_DONE,
        INSTALL_ACTIVE_CHANGED,
    },
    util::core_log::PluginCoreLog,
};

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
        inbox: CbReceiver<Arc<ErasedTopic>>,
    ) -> AdapterResult {
        let (stop_tx, stop_rx) = bounded::<()>(1);

        // Loggers: StreamDeck for adapter logs, CoreLog wrapper for core calls
        let sd_log: Arc<dyn ActionLog> = cx.log().clone();
        let core_log: Arc<dyn CoreLog> = Arc::new(PluginCoreLog(sd_log.clone()));

        let installs = cx
            .try_ext::<InstallPaths>()
            .ok_or(AdapterError::Init("InstallPaths ext missing".to_string()))?
            .clone();

        let store = cx
            .try_ext::<ActionBindingsStore>()
            .ok_or(AdapterError::Init(
                "ActionBindingsStore ext missing".to_string(),
            ))?
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
            info!(sd_log, "BindingsAdapter started");

            loop {
                select! {
                    recv(inbox) -> msg => match msg {
                        Ok(ev) => {
                            // ─────────────────────────────────────────────────────────────
                            // Rebuild from XML + Save profile & cache
                            // ─────────────────────────────────────────────────────────────
                            if let Some(m) = ev.downcast(BINDINGS_REBUILD_AND_SAVE) {
                                let Some(game_root) = installs.get(m.ty) else {
                                    warn!(sd_log, "no install path for {:?}", m.ty);
                                    continue;
                                };
                                debug!(sd_log, "BINDINGS_REBUILD_AND_SAVE for {:?}", m.ty);

                                // Parse from files via core
                                let mut ab = match parse_bindings_from_install(
                                    &res_dir.get(),
                                    &game_root,
                                    m.with_custom,
                                    &core_log,
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        warn!(sd_log, "parse_bindings_from_install({:?}): {}", m.ty, e);
                                        continue;
                                    }
                                };

                                // Fill gaps (CoreLog)
                                ab.generate_missing_binds(&core_log);

                                // Write XML profile + JSON cache via core
                                if let Err(e) = save_bindings_profile_and_cache(
                                    &ab,
                                    &game_root,
                                    plugin_id,
                                    m.ty,
                                    m.name.as_deref(),
                                    None,        // devices (defaults to kb=1, mouse=1)
                                    &core_log,
                                ) {
                                    warn!(sd_log, "save_bindings_profile_and_cache: {}", e);
                                }

                                // Publish snapshot
                                debug!(sd_log, "Storing ActionBindings in store");
                                store.replace(ab);
                                bus.publish_t(BINDINGS_PARSED, ());
                                continue;
                            }

                            // ─────────────────────────────────────────────────────────────
                            // First-time load after initial scan: prefer cache
                            // ─────────────────────────────────────────────────────────────
                            if ev.downcast(INITIAL_INSTALL_SCAN_DONE).is_some() {
                                let ty = active_install.get();
                                debug!(sd_log, "INITIAL_INSTALL_SCAN_DONE: {:?}", ty);

                                let ab = match load_bindings_from_appdata(
                                    plugin_id,
                                    ty,
                                    &core_log,
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        warn!(sd_log, "load_bindings_from_appdata: {}", e);
                                        continue;
                                    }
                                };

                                debug!(sd_log, "Storing ActionBindings in store");
                                store.replace(ab);
                                continue;
                            }

                            // ─────────────────────────────────────────────────────────────
                            // Active install changed: try cache, else parse files
                            // ─────────────────────────────────────────────────────────────
                            if let Some(m) = ev.downcast(INSTALL_ACTIVE_CHANGED) {
                                store.clear();
                                debug!(sd_log, "INSTALL_ACTIVE_CHANGED -> {:?}", m.ty);

                                // 1) Try cache
                                let mut ab = match load_bindings_from_appdata(
                                    plugin_id,
                                    m.ty,
                                    &core_log,
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        warn!(sd_log, "cache miss for {:?}: {}", m.ty, e);
                                        ActionBindings::default()
                                    }
                                };

                                // 2) Fallback to parse
                                if ab.action_maps.is_empty() {
                                    let Some(game_root) = installs.get(m.ty) else {
                                        warn!(sd_log, "no install path for {:?}", m.ty);
                                        continue;
                                    };
                                    match parse_bindings_from_install(
                                        &res_dir.get(),
                                        &game_root,
                                        true, // include_custom
                                        &core_log,
                                    ) {
                                        Ok(v) => ab = v,
                                        Err(e) => {
                                            warn!(sd_log, "parse fallback {:?}: {}", m.ty, e);
                                            continue;
                                        }
                                    }
                                }

                                debug!(sd_log, "Storing ActionBindings in store");
                                store.replace(ab);
                                continue;
                            }

                            // else: not for us
                        }
                        Err(e) => error!(sd_log, "recv: {}", e),
                    },

                    recv(stop_rx) -> _ => break,
                }
            }

            info!(sd_log, "BindingsAdapter stopped");
        });

        Ok(AdapterHandle::from_crossbeam(join, stop_tx))
    }
}
