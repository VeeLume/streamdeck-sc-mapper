use std::env;
use std::path::PathBuf;

pub(crate) fn get_resource_dir() -> Result<PathBuf, String> {
    match env::current_exe() {
        Ok(path) => match path.parent() {
            Some(parent) => Ok(parent.to_path_buf()),
            None => Err("Failed to get parent directory of current executable".to_string()),
        },
        Err(e) => Err(format!("Failed to get current executable path: {e}")),
    }
}
