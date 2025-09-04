use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use streamdeck_sc_core::prelude::GameInstallType;

/// Install paths (per type).
#[derive(Clone, Default)]
pub struct InstallPaths(Arc<RwLock<HashMap<GameInstallType, Option<PathBuf>>>>);

impl InstallPaths {
    pub fn get(&self, ty: GameInstallType) -> Option<PathBuf> {
        self.0
            .read()
            .ok()
            .and_then(|m| m.get(&ty).cloned().unwrap_or(None))
    }
    pub fn replace_all(&self, m: HashMap<GameInstallType, Option<PathBuf>>) {
        if let Ok(mut w) = self.0.write() {
            *w = m;
        }
    }
}
