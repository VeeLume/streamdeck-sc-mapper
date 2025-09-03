use crate::sc::topics::{INSTALL_SCAN, INSTALL_UPDATED};
use crate::sc::{
    shared::{ActiveInstall, GameInstallType, InstallPaths},
    topics::{INITIAL_INSTALL_SCAN_DONE, INSTALL_ACTIVE_CHANGED, InstallActiveChanged},
};
use crossbeam_channel::{Receiver as CbReceiver, bounded, select};
use regex::Regex;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use streamdeck_lib::prelude::*;

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

pub fn scan_paths_and_active() -> Result<
    (
        HashMap<GameInstallType, Option<PathBuf>>,
        Option<GameInstallType>,
    ),
    String,
> {
    use directories::BaseDirs;

    let log_file = BaseDirs::new()
        .ok_or("no data dir")?
        .data_dir()
        .join("rsilauncher")
        .join("logs")
        .join("log.log");

    if !log_file.try_exists().unwrap_or(false) {
        return Err(format!("launcher log not found at {}", log_file.display()));
    }
    let content = std::fs::read_to_string(&log_file).map_err(|e| e.to_string())?;

    // Plain “Launching … from (…)” lines per channel
    let live = Regex::new(r#"Launching Star Citizen LIVE from \((.+)\)"#).unwrap();
    let ptu = Regex::new(r#"Launching Star Citizen PTU from \((.+)\)"#).unwrap();
    let tech = Regex::new(r#"Launching Star Citizen Tech Preview from \((.+)\)"#).unwrap();

    // Unified matcher with optional “[Launcher::launch] ” prefix
    let launch_line = Regex::new(
        r#"(?:\[Launcher::launch\]\s+)?Launching Star Citizen (LIVE|PTU|Tech Preview) from \((.+)\)"#
    ).unwrap();

    let mut found: HashMap<GameInstallType, PathBuf> = HashMap::new();
    let mut last_active: Option<GameInstallType> = None;

    for line in content.lines() {
        // Capture install roots (and consider these as “active” moments too)
        if let Some(c) = live.captures(line).and_then(|c| c.get(1)) {
            found.insert(GameInstallType::Live, PathBuf::from(c.as_str()));
            last_active = Some(GameInstallType::Live);
        }
        if let Some(c) = ptu.captures(line).and_then(|c| c.get(1)) {
            found.insert(GameInstallType::Ptu, PathBuf::from(c.as_str()));
            last_active = Some(GameInstallType::Ptu);
        }
        if let Some(c) = tech.captures(line).and_then(|c| c.get(1)) {
            found.insert(GameInstallType::TechPreview, PathBuf::from(c.as_str()));
            last_active = Some(GameInstallType::TechPreview);
        }

        // Also match the variant that includes “[Launcher::launch] …”
        if let Some(caps) = launch_line.captures(line) {
            last_active = match caps.get(1).map(|m| m.as_str()) {
                Some("LIVE") => Some(GameInstallType::Live),
                Some("PTU") => Some(GameInstallType::Ptu),
                Some("Tech Preview") => Some(GameInstallType::TechPreview),
                _ => last_active,
            };
        }
    }

    // Normalize to output shape
    let mut out: HashMap<GameInstallType, Option<PathBuf>> = HashMap::new();
    for ty in GameInstallType::ALL {
        out.insert(ty, found.get(&ty).cloned());
    }

    Ok((out, last_active))
}
