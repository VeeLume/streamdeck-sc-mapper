use crossbeam_channel::{Receiver as CbReceiver, bounded, select};
use std::sync::Arc;
use streamdeck_lib::prelude::*;
use streamdeck_sc_core::prelude::{GameInstallType, scan_paths_and_active};

use crate::{
    state::{active_install_store::ActiveInstall, install_paths_store::InstallPaths},
    topics::{
        INITIAL_INSTALL_SCAN_DONE, INSTALL_ACTIVE_CHANGED, INSTALL_SCAN, INSTALL_UPDATED,
        InstallActiveChanged,
    },
};

pub struct InstallScannerAdapter;

impl InstallScannerAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl AdapterStatic for InstallScannerAdapter {
    const NAME: &'static str = "sc.install_scanner";
}

impl Adapter for InstallScannerAdapter {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn policy(&self) -> StartPolicy {
        StartPolicy::Eager
    }

    fn topics(&self) -> &'static [&'static str] {
        &[INSTALL_SCAN.name]
    }

    fn start(
        &self,
        cx: &Context,
        bus: std::sync::Arc<dyn Bus>,
        inbox: CbReceiver<Arc<ErasedTopic>>,
    ) -> AdapterResult {
        let (stop_tx, stop_rx) = bounded::<()>(1);
        let cx = cx.clone();
        let logger = cx.log().clone();
        let store = cx
            .try_ext::<InstallPaths>()
            .ok_or(AdapterError::Init(
                "InstallPaths extension missing".to_string(),
            ))?
            .clone();
        let active = cx
            .try_ext::<ActiveInstall>()
            .ok_or(AdapterError::Init(
                "ActiveInstall extension missing".to_string(),
            ))?
            .clone();

        let join = std::thread::spawn(move || {
            info!(logger, "InstallScannerAdapter started");

            let do_scan = || {
                match scan_paths_and_active() {
                    Ok((map, active_now)) => {
                        // update paths map
                        store.replace_all(map);
                        bus.publish_t(INSTALL_UPDATED, ());

                        // only emit if changed
                        let new_ty = active_now.unwrap_or(GameInstallType::Live);
                        if active.get() != new_ty {
                            active.set(new_ty);
                            bus.publish_t(
                                INSTALL_ACTIVE_CHANGED,
                                InstallActiveChanged { ty: new_ty },
                            );
                        }
                    }
                    Err(e) => warn!(logger, "scan_paths_and_active: {}", e),
                }
            };

            // initial scan
            do_scan();
            bus.publish_t(INITIAL_INSTALL_SCAN_DONE, ());
            debug!(logger, "Initial install scan done");

            loop {
                select! {
                    recv(inbox) -> msg => {
                        match msg {
                            Ok(ev) if ev.downcast(INSTALL_SCAN).is_some() => {
                                debug!(logger, "manual install scan");
                                do_scan();
                            }
                            Ok(_) => {}
                            Err(e) => error!(logger, "recv error: {}", e),
                        }
                    }
                    recv(stop_rx) -> _ => break,
                }
            }

            info!(logger, "InstallScannerAdapter stopped");
        });

        Ok(AdapterHandle::from_crossbeam(join, stop_tx))
    }
}
