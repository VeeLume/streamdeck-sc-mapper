use std::sync::{Arc, RwLock};
use streamdeck_sc_core::prelude::GameInstallType;

/// Track currently active install.
#[derive(Clone, Default)]
pub struct ActiveInstall(Arc<RwLock<GameInstallType>>);

impl ActiveInstall {
    pub fn get(&self) -> GameInstallType {
        self.0.read().map(|g| *g).unwrap_or(GameInstallType::Live)
    }
    pub fn set(&self, v: GameInstallType) {
        if let Ok(mut w) = self.0.write() {
            *w = v;
        }
    }
}
