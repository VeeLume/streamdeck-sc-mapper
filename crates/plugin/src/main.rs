use std::{process::exit, sync::Arc};

use streamdeck_lib::prelude::*;

use crate::{
    actions::{
        generate_profile::GenerateProfileAction, rotate_install::RotateInstallAction,
        sc_action::ScAction,
    },
    adapters::{
        bindings_adapter::BindingsAdapter, exec_adapter::ExecAdapter,
        install_scanner::InstallScannerAdapter,
    },
    state::{
        action_bindings_store::ActionBindingsStore, active_install_store::ActiveInstall,
        install_paths_store::InstallPaths, resource_dir_store::ResourceDir,
    },
};

mod actions;
mod adapters;
mod simulate;
mod state;
mod topics;
mod util;

const PLUGIN_ID: &str = "icu.veelume.sc-mapper";

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
            error!(logger, "Failed to parse launch args: {e}");
            exit(2);
        }
    };

    let resource_dir = match util::resource_dir::get_resource_dir() {
        Ok(dir) => dir,
        Err(e) => {
            error!(logger, "Failed to get resource dir: {e}");
            exit(3);
        }
    };

    let hooks = AppHooks::default().append(|cx, ev| {
        use streamdeck_lib::prelude::HookEvent::*;
        match ev {
            ApplicationDidLaunch { .. } => {
                cx.bus().publish_t(topics::INSTALL_SCAN, ());
            }
            _ => {
                debug!(cx.log(), "HookEvent: {:?}", ev);
            }
        }
    });

    let plugin = match PluginBuilder::new()
        .set_hooks(hooks)
        .add_extension(Arc::new(ActionBindingsStore::new(logger.clone())))
        .add_extension(Arc::new(ResourceDir::new(resource_dir)))
        .add_extension(Arc::new(InstallPaths::default()))
        .add_extension(Arc::new(ActiveInstall::default()))
        .add_adapter(InstallScannerAdapter::new())
        .add_adapter(BindingsAdapter::new(PLUGIN_ID))
        .add_adapter(ExecAdapter::new())
        .add_action(ActionFactory::default_of::<GenerateProfileAction>())
        .add_action(ActionFactory::default_of::<RotateInstallAction>())
        .add_action(ActionFactory::default_of::<ScAction>())
        .build()
    {
        Ok(p) => p,
        Err(e) => {
            error!(logger, "Failed to build plugin: {e}");
            exit(4);
        }
    };

    let cfg = RunConfig::default().set_log_websocket(false);

    match run(plugin, args, logger.clone(), cfg) {
        Ok(_) => {
            info!(logger, "Plugin exited successfully.");
        }
        Err(e) => {
            error!(logger, "Plugin run failed: {e}");
            exit(5);
        }
    }
}
