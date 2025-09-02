use crate::actions::generate_profile::GenerateProfileAction;
use crate::actions::rotate_install::RotateInstallAction;
use crate::actions::sc_action::ScAction;
use crate::sc::adapters::bindings_adapter::BindingsAdapter;
use crate::sc::adapters::exec_adapter::ExecAdapter;
use crate::sc::adapters::install_scanner::InstallScannerAdapter;
use crate::sc::shared::{ActiveInstall, InstallPaths};
use crate::{bindings::action_bindings::ActionBindingsStore, sc::shared::ResourceDir};
use std::env;
use std::process::exit;
use std::{path::PathBuf, sync::Arc};
use streamdeck_lib::prelude::*;

mod data_source;
mod serde_helpers;
mod bindings {
    mod action_binding;
    pub mod action_bindings;
    mod action_map;
    mod activation_mode;
    mod bind;
    mod bind_tokens;
    mod binds;
    mod binds_generator;
    pub mod constants;
    mod generate_mappings_xml;
    mod helpers;
    mod str_intern;
    pub mod translations;
}
mod sc {
    pub mod shared;
    pub mod topics;
    pub mod adapters {
        pub mod bindings_adapter;
        pub mod exec_adapter;
        pub mod install_scanner;
    }
}
mod actions {
    pub mod generate_profile;
    pub mod rotate_install;
    pub mod sc_action;
}
const PLUGIN_ID: &str = "icu.veelume.sc-mapper";

fn get_resource_dir() -> Result<PathBuf, String> {
    match env::current_exe() {
        Ok(path) => match path.parent() {
            Some(parent) => Ok(parent.to_path_buf()),
            None => Err("Failed to get parent directory of current executable".to_string()),
        },
        Err(e) => Err(format!("Failed to get current executable path: {e}")),
    }
}

fn main() {
    let logger: Arc<dyn ActionLog> = match FileLogger::from_appdata(PLUGIN_ID) {
        Ok(logger) => Arc::new(logger),
        Err(e) => {
            eprintln!("Failed to create logger: {e}");
            exit(1);
        }
    };

    let args = match parse_launch_args() {
        Ok(args) => args,
        Err(e) => {
            error!(logger, "Failed to parse launch arguments: {}", e);
            exit(2);
        }
    };

    let hooks = AppHooks::default().append(|cx, ev| {
        match ev {
            // ---- lifecycle you previously logged + kicked scans on ----
            HookEvent::Init => {
                info!(cx.log(), "Plugin initialized with ID: {}", PLUGIN_ID);
                // discover installs (BindingsAdapter listens for INSTALL_SCAN)
            }
            HookEvent::ApplicationDidLaunch(app) => {
                info!(cx.log(), "Application launched: {:?}", app);
                cx.bus().publish_t(crate::sc::topics::INSTALL_SCAN, ());
            }
            HookEvent::ApplicationDidTerminate(app) => {
                info!(cx.log(), "Application terminated: {:?}", app);
            }

            // ---- optional SD lifecycle logs (kept for parity) ----
            HookEvent::DeviceDidConnect(dev, info) => {
                info!(cx.log(), "Device connected: {} ({:?})", dev, info);
            }
            HookEvent::DeviceDidDisconnect(dev) => {
                info!(cx.log(), "Device disconnected: {}", dev);
            }
            HookEvent::DeviceDidChange(dev, info) => {
                info!(cx.log(), "Device changed: {} ({:?})", dev, info);
            }
            HookEvent::DidReceiveDeepLink(url) => {
                info!(cx.log(), "Deep link: {}", url);
            }
            HookEvent::DidReceiveGlobalSettings(_gs) => {
                // already applied in main loop; log if you want:
                // info!(cx.log(), "Global settings received");
            }

            // ---- runtime mirrors ----
            HookEvent::Outgoing(_msg) => {
                // if you want to log SD outbound:
                // debug!(cx.log(), "→ {:?}", msg);
            }
            HookEvent::Log(_level, _msg) => {}

            // ---- typed notifies ----
            HookEvent::ActionNotify(ev) => {
                info!(cx.log(), "Action notify topic={}", ev.name(),);
            }
            HookEvent::AdapterNotify(target, ev) => {
                info!(
                    cx.log(),
                    "Adapter notify target={:?} topic={}",
                    target,
                    ev.name(),
                );
                // example: if you ever want to react to only certain targets:
                // if matches!(target, AdapterTarget::Topic(t) if *t == crate::sc::topics::INSTALL_SCAN.name) { ... }
            }

            // ---- control + tick/exit (kept for completeness) ----
            HookEvent::AdapterControl(ctl) => {
                debug!(cx.log(), "Adapter control: {:?}", ctl);
            }
            HookEvent::Tick => { /* periodic */ }
            HookEvent::Exit => {
                info!(cx.log(), "Exiting");
            }

            // raw incoming (catch-all if you want)
            HookEvent::Incoming(_ev) => { /* already handled upstream */ }

            _ => {
                debug!(cx.log(), "Unhandled hook event: {:?}", ev);
            }
        }
    });

    let action_bindings = ActionBindingsStore::new(logger.clone());

    let resource_dir = match get_resource_dir() {
        Ok(dir) => ResourceDir::new(dir),
        Err(e) => {
            error!(logger, "Failed to get resource directory: {}", e);
            exit(3);
        }
    };
    let install_paths = InstallPaths::default();
    let active_install = ActiveInstall::default();

    let plugin = match PluginBuilder::new()
        .set_hooks(hooks)
        .add_extension(Arc::new(action_bindings))
        .add_extension(Arc::new(resource_dir))
        .add_extension(Arc::new(install_paths))
        .add_extension(Arc::new(active_install))
        .add_adapter(InstallScannerAdapter::new())
        .add_adapter(BindingsAdapter::new(PLUGIN_ID))
        .add_adapter(ExecAdapter::new())
        .add_action(ActionFactory::default_of::<GenerateProfileAction>())
        .add_action(ActionFactory::default_of::<ScAction>())
        .add_action(ActionFactory::default_of::<RotateInstallAction>())
        .build()
    {
        Ok(plugin) => plugin,
        Err(e) => {
            error!(logger, "Failed to build plugin: {}", e);
            exit(3);
        }
    };

    let cfg = RunConfig::default().set_log_websocket(false);

    match run(plugin, args, logger.clone(), cfg) {
        Ok(_) => {
            info!(logger, "Plugin exited successfully.");
        }
        Err(e) => {
            error!(logger, "Plugin run failed: {}", e);
            exit(4);
        }
    }
}
