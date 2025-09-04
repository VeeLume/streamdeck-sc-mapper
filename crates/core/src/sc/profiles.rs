use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Local;

use crate::CoreLog;
use crate::bindings::{
    action_bindings::ActionBindings,
    constants::{ACTION_MAP_UI_CATEGORIES, SKIP_ACTION_MAPS},
};
use crate::sc::install::GameInstallType;

/// Join `<install root>/user/client/0/Profiles/default/actionmaps.xml` and return it
/// if the file exists. Otherwise returns `None`.
pub fn resolve_custom_profile_from_root(root: &Path) -> Option<PathBuf> {
    let p = root
        .join("user")
        .join("client")
        .join("0")
        .join("Profiles")
        .join("default")
        .join("actionmaps.xml");
    if p.is_file() { Some(p) } else { None }
}

/// Load ActionBindings from the game’s default profile, and (optionally) overlay the custom profile.
/// This uses the shared skip/category constants and rebuilds the activation indexes.
pub fn parse_bindings_from_install(
    resource_dir: &Path, // where defaultProfile.xml lives
    game_root: &Path,    // install root for resolving custom
    include_custom: bool,
    logger: &Arc<dyn CoreLog>,
) -> Result<ActionBindings, String> {
    let default_profile = resource_dir.join("defaultProfile.xml");
    if !default_profile.is_file() {
        return Err(format!(
            "default profile not found at {}",
            default_profile.display()
        ));
    }

    let custom_file = if include_custom {
        resolve_custom_profile_from_root(game_root)
    } else {
        None
    };

    let mut ab = ActionBindings::default();
    ab.load_default_profile(
        &default_profile,
        &SKIP_ACTION_MAPS,
        &ACTION_MAP_UI_CATEGORIES,
        logger,
    )?;

    if let Some(cf) = custom_file {
        if cf.try_exists().unwrap_or(false) {
            if let Err(e) = ab.apply_custom_profile(&cf, logger) {
                logger.warn(&format!("apply_custom_profile({}): {}", cf.display(), e));
            }
        } else {
            logger.debug(&format!("no custom file at {}", cf.display()));
        }
    }

    ab.activation.rebuild_indexes();
    Ok(ab)
}

/// Compute the AppData folder for this plugin.
pub fn appdata_dir(plugin_id: &str) -> Result<PathBuf, String> {
    let base = directories::BaseDirs::new().ok_or("Could not find user data directory")?;
    let dir = base.data_dir().join(plugin_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Return `<AppData>/<plugin_id>/bindings_<INSTALL>.json` (does not create it).
pub fn bindings_cache_path(plugin_id: &str, ty: GameInstallType) -> Result<PathBuf, String> {
    Ok(appdata_dir(plugin_id)?.join(format!("bindings_{}.json", ty.name())))
}

/// Load ActionBindings from the AppData JSON cache for an install.
pub fn load_bindings_from_appdata(
    plugin_id: &str,
    ty: GameInstallType,
    logger: &Arc<dyn CoreLog>,
) -> Result<ActionBindings, String> {
    let file = bindings_cache_path(plugin_id, ty)?;
    if !file.try_exists().unwrap_or(false) {
        return Err(format!("No bindings file at {}", file.display()));
    }
    let content =
        fs::read_to_string(&file).map_err(|e| format!("read {}: {}", file.display(), e))?;
    let ab = ActionBindings::from_json(&content, logger)?;
    logger.info(&format!(
        "Loaded {} action maps with {} activation modes for {}",
        ab.action_maps.len(),
        ab.activation.len(),
        ty.name()
    ));
    Ok(ab)
}

/// Write mappings XML under `<install>/user/client/0/controls/mappings/<plugin_id>.xml`
/// and write the JSON cache to `<AppData>/<plugin_id>/bindings_<INSTALL>.json`.
/// Returns `(xml_path, json_path)`.
pub fn save_bindings_profile_and_cache(
    ab: &ActionBindings,
    game_root: &Path,
    plugin_id: &str,
    ty: GameInstallType,
    profile_name: Option<&str>,
    devices: Option<&[(&str, &str)]>,
    logger: &Arc<dyn CoreLog>,
) -> Result<(PathBuf, PathBuf), String> {
    // 1) Write mappings XML inside the install
    let profile_dir = game_root
        .join("user")
        .join("client")
        .join("0")
        .join("controls")
        .join("mappings");
    fs::create_dir_all(&profile_dir)
        .map_err(|e| format!("create {}: {}", profile_dir.display(), e))?;

    let name = profile_name.map(|s| s.to_string()).unwrap_or_else(|| {
        format!(
            "SC-Mapper-{}-{}",
            ty.name(),
            Local::now().format("%Y%m%d-%H:%M")
        )
    });

    let xml_path = profile_dir.join(format!("{plugin_id}.xml"));
    ab.generate_mapping_xml(&xml_path, devices, &name)
        .map_err(|e| format!("generate_mapping_xml {}: {}", xml_path.display(), e))?;
    logger.info(&format!("wrote profile {}", xml_path.display()));

    // 2) Write JSON cache to AppData
    let json_path = bindings_cache_path(plugin_id, ty)?;
    let json = ab
        .to_json()
        .map_err(|e| format!("serialize bindings: {}", e))?;
    fs::write(&json_path, json).map_err(|e| format!("write {}: {}", json_path.display(), e))?;
    logger.info(&format!(
        "wrote {}",
        json_path.file_name().unwrap().to_string_lossy()
    ));

    Ok((xml_path, json_path))
}
