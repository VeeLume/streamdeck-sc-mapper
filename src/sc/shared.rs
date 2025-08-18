use std::{ collections::HashMap, path::PathBuf, sync::{ Arc, RwLock } };
use directories::BaseDirs;
use serde::{ Deserialize, Serialize };


#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum GameInstallType {
    #[default]
    Live,
    Ptu,
    TechPreview,
}
impl GameInstallType {
    pub fn name(&self) -> &'static str {
        match self {
            GameInstallType::Live => "LIVE",
            GameInstallType::Ptu => "PTU",
            GameInstallType::TechPreview => "TechPreview",
        }
    }

    pub const ALL: [GameInstallType; 3] = [
        GameInstallType::Live,
        GameInstallType::Ptu,
        GameInstallType::TechPreview,
    ];

    pub fn iter() -> impl Iterator<Item = GameInstallType> {
        Self::ALL.into_iter()
    }
}

#[derive(Clone)]
pub struct ActiveInstall(Arc<RwLock<GameInstallType>>);
impl Default for ActiveInstall {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(GameInstallType::Live)))
    }
}
impl ActiveInstall {
    pub fn get(&self) -> GameInstallType {
        self.0
            .read()
            .map(|g| *g)
            .unwrap_or(GameInstallType::Live)
    }
    pub fn set(&self, v: GameInstallType) {
        if let Ok(mut w) = self.0.write() {
            *w = v;
        }
    }
}

/// One path where resources live (global.ini, defaultProfile.xml)
#[derive(Clone)]
pub struct ResourceDir(Arc<RwLock<PathBuf>>);
impl ResourceDir {
    pub fn new(dir: PathBuf) -> Self {
        Self(Arc::new(RwLock::new(dir)))
    }
    pub fn get(&self) -> PathBuf {
        self.0
            .read()
            .ok()
            .map(|p| p.clone())
            .unwrap_or_default()
    }
}

/// Map of install type -> discovered game folder (may be None)
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

/// AppData path helper
pub fn appdata_dir(plugin_id: &str) -> Result<PathBuf, String> {
    let base = BaseDirs::new().ok_or("Could not find user data directory")?;
    let p = base.data_dir().join(plugin_id);
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    Ok(p)
}
