// src/actions/rotate_install.rs
use constcat::concat;
use streamdeck_lib::prelude::*;
use streamdeck_sc_core::prelude::GameInstallType;

use crate::{
    PLUGIN_ID,
    adapters::bindings_adapter::BindingsAdapter,
    state::{active_install_store::ActiveInstall, install_paths_store::InstallPaths},
    topics::{INSTALL_ACTIVE_CHANGED, InstallActiveChanged},
};

#[derive(Default)]
pub struct RotateInstallAction;

impl ActionStatic for RotateInstallAction {
    const ID: &'static str = concat!(PLUGIN_ID, ".rotate-install");
}

impl Action for RotateInstallAction {
    fn id(&self) -> &str {
        Self::ID
    }

    fn init(&mut self, cx: &Context, ctx: &str) {
        info!(cx.log(), "RotateInstallAction init for {}", ctx);
    }

    fn will_appear(&mut self, cx: &Context, ev: &WillAppear) {
        // Set title to current active install type
        let active = match cx.try_ext::<ActiveInstall>() {
            Some(a) => a.get(),
            None => GameInstallType::Live, // Default to Live if not set
        };
        let title = active.name().to_string();
        cx.sd().set_title(ev.context, Some(title), None, None);
    }

    fn did_receive_settings(&mut self, _cx: &Context, _ev: &DidReceiveSettings) {}

    fn on_notify(&mut self, cx: &Context, ctx_id: &str, event: &ErasedTopic) {
        if let Some(m) = event.downcast(INSTALL_ACTIVE_CHANGED) {
            // Update title when active install changes
            cx.sd()
                .set_title(ctx_id, Some(m.ty.name().to_string()), None, None);
        }
    }

    fn key_down(&mut self, cx: &Context, ev: &KeyDown) {
        let installs = match cx.try_ext::<InstallPaths>() {
            Some(s) => s.clone(),
            None => {
                cx.sd().show_alert(ev.context);
                return;
            }
        };
        let active = match cx.try_ext::<ActiveInstall>() {
            Some(a) => a.clone(),
            None => {
                cx.sd().show_alert(ev.context);
                return;
            }
        };

        // Gather available types in a stable order
        let available: Vec<GameInstallType> = GameInstallType::iter()
            .filter(|ty| installs.get(*ty).is_some())
            .collect();

        if available.is_empty() {
            cx.sd().show_alert(ev.context);
            return;
        }

        // Find next
        let cur = active.get();
        let next = if let Some(pos) = available.iter().position(|&ty| ty == cur) {
            // Wrap around to the first if at the end
            available
                .get((pos + 1) % available.len())
                .cloned()
                .unwrap_or(cur)
        } else {
            // If current is not found, default to the first available
            available.first().cloned().unwrap_or(cur)
        };

        // Set and broadcast
        active.set(next);
        // ⬇ important: notify adapters (BindingsAdapter listens for this)
        cx.bus().adapters_notify_name_of::<BindingsAdapter, _>(
            INSTALL_ACTIVE_CHANGED,
            InstallActiveChanged { ty: next },
        );

        // Small UX ping
        cx.sd()
            .set_title(ev.context, Some(next.name().to_string()), None, None);
    }

    fn key_up(&mut self, _cx: &Context, _ev: &KeyUp) {}
}
