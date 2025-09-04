use std::{path::PathBuf, sync::Arc};

/// Resource dir (global.ini, defaultProfile.xml).
#[derive(Clone)]
pub struct ResourceDir(Arc<PathBuf>);

impl ResourceDir {
    pub fn new(dir: PathBuf) -> Self {
        Self(Arc::new(dir))
    }
    pub fn get(&self) -> PathBuf {
        (*self.0).clone()
    }
}
