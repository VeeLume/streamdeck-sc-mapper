use std::{ env, fs };
use std::{ collections::HashMap, path::PathBuf, sync::Arc };
use directories::BaseDirs;
use regex::Regex;

use crate::action_binds::constants::{ ACTION_MAP_UI_CATEGORIES, SKIP_ACTION_MAPS };
use crate::data_source::DataSourceResult;
use crate::logger::ActionLog;
use crate::action_binds::action_bindings::ActionBindings;
use crate::plugin::PLUGIN_UUID;

fn parse_translation_line(line: &str) -> Option<(&str, &str)> {
    if let Some(idx) = line.find(",P=") {
        let (key, value) = line.split_at(idx);
        Some((key.trim(), value.trim_start_matches(",P=").trim()))
    } else if let Some(idx) = line.find(',') {
        let (key, value) = line.split_at(idx);
        Some((key.trim(), value.trim_start_matches(',').trim()))
    } else if let Some(idx) = line.find('=') {
        let (key, value) = line.split_at(idx);
        Some((key.trim(), value.trim_start_matches('=').trim()))
    } else {
        None
    }
}

fn get_appdata_dir() -> Result<PathBuf, String> {
    let base = BaseDirs::new().ok_or("Could not find user data directory")?;
    let app_dir = base.data_dir().join(PLUGIN_UUID).to_path_buf();
    fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create app data directory: {e}"))?;
    Ok(app_dir)
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum GameInstallType {
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
}

pub struct AppState {
    pub resource_dir: PathBuf,
    pub logger: Arc<dyn ActionLog>,

    pub game_paths: HashMap<GameInstallType, Option<PathBuf>>,
    pub action_bindings: HashMap<GameInstallType, ActionBindings>,
    pub translations: HashMap<GameInstallType, HashMap<String, String>>,
    pub cached_data_sources: HashMap<GameInstallType, Option<Vec<DataSourceResult>>>,
}

impl AppState {
    pub fn new(logger: Arc<dyn ActionLog>) -> Result<Self, String> {
        let game_paths = GameInstallType::ALL.iter()
            .map(|&ty| (ty, None))
            .collect();

        let action_bindings = GameInstallType::ALL.iter()
            .map(|&ty| (ty, ActionBindings::new(logger.clone())))
            .collect();

        let translations = GameInstallType::ALL.iter()
            .map(|&ty| (ty, HashMap::new()))
            .collect();

        let cached_data_sources = GameInstallType::ALL.iter()
            .map(|&ty| (ty, None))
            .collect();

        let resource_dir = {
            match env::current_exe() {
                Ok(path) =>
                    match path.parent() {
                        Some(parent) => parent.to_path_buf(),
                        None => {
                            logger.log("❌ Failed to get parent directory of current executable");
                            return Err(
                                "Failed to get parent directory of current executable".to_string()
                            );
                        }
                    }
                Err(e) => {
                    logger.log(&format!("❌ Failed to get current executable path: {e}"));
                    return Err(format!("Failed to get current executable path: {e}"));
                }
            }
        };

        Ok(Self {
            resource_dir,
            logger,
            game_paths,
            action_bindings,
            translations,
            cached_data_sources,
        })
    }

    pub fn initialize(&mut self) {
        self.logger.log("📦 Initializing AppState...");

        self.load_game_paths();
        self.logger.log(&format!("📂 Game paths: {:?}", self.game_paths));

        for ty in GameInstallType::ALL {
            self.logger.log(&format!("🔄 Loading data for {:?}", ty));
            self.load_translations(ty);
            self.load_action_bindings(ty);
        }
    }

    pub fn load_game_paths(&mut self) {
        self.logger.log("🔍 Loading game paths...");

        let log_file = {
            match BaseDirs::new() {
                Some(base) =>
                    base.data_dir().join("rsilauncher").join("logs").join("log.log").to_path_buf(),
                None => {
                    self.logger.log("❌ Could not find user data directory");
                    return;
                }
            }
        };

        if !fs::exists(&log_file).unwrap_or(false) {
            self.logger.log(&format!("⚠️ Log file not found at {}", log_file.display()));
            return;
        }

        let content = {
            match fs::read_to_string(log_file) {
                Ok(content) => content,
                Err(e) => {
                    self.logger.log(&format!("❌ Failed to read log file: {e}"));
                    return;
                }
            }
        };

        let live_regex = Regex::new(r#"Launching Star Citizen LIVE from \((.+)\)"#).unwrap();
        let ptu_regex = Regex::new(r#"Launching Star Citizen PTU from \((.+)\)"#).unwrap();
        let tech_preview_regex = Regex::new(
            r#"Launching Star Citizen Tech Preview from \((.+)\)"#
        ).unwrap();

        let patterns: Vec<(&Regex, GameInstallType)> = vec![
            (&live_regex, GameInstallType::Live),
            (&ptu_regex, GameInstallType::Ptu),
            (&tech_preview_regex, GameInstallType::TechPreview)
        ];

        let mut found_paths: HashMap<GameInstallType, PathBuf> = HashMap::new();

        for line in content.lines() {
            for (regex, ty) in &patterns {
                if let Some(caps) = regex.captures(line) {
                    if let Some(path) = caps.get(1) {
                        found_paths.insert(*ty, PathBuf::from(path.as_str()));
                        break; // Found a match, no need to check other patterns
                    }
                }
            }
        }

        for ty in GameInstallType::ALL {
            self.game_paths.insert(ty, found_paths.get(&ty).cloned());
        }

        self.logger.log(&format!("✅ Loaded game paths: {:?}", self.game_paths));
    }

    pub fn load_translations(&mut self, ty: GameInstallType) {
        if self.game_paths.get(&ty).is_none() {
            self.logger.log(&format!("❌ No game path found for {:?}", ty));
            return;
        }

        let mut translation_file = self.resource_dir.clone();
        translation_file.push("global.ini");

        if !fs::exists(&translation_file).unwrap_or(false) {
            self.logger.log(
                &format!("⚠️ Translation file not found at {}", translation_file.display())
            );
            return;
        }

        let content = {
            match fs::read_to_string(&translation_file) {
                Ok(content) => content,
                Err(e) => {
                    self.logger.log(&format!("❌ Failed to read translation file: {e}"));
                    return;
                }
            }
        };

        let mut translations = HashMap::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                continue; // Skip empty lines and comments
            }

            if let Some((key, value)) = parse_translation_line(trimmed) {
                translations.insert(key.to_string(), value.to_string());
            }
        }

        self.translations.insert(ty, translations);
        self.logger.log(&format!("🈯 Translations for {:?} loaded", ty));
    }

    pub fn load_action_bindings(&mut self, ty: GameInstallType) {
        if self.game_paths.get(&ty).is_none() {
            self.logger.log(&format!("❌ No game path found for {:?}", ty));
            return;
        }

        let bindings_file = {
            match get_appdata_dir() {
                Ok(base) => {
                    let mut path = base;
                    path.push(format!("bindings_{}.json", ty.name()));
                    path
                }
                Err(e) => {
                    self.logger.log(&format!("❌ Failed to get AppData directory: {e}"));
                    return;
                }
            }
        };

        if !fs::exists(&bindings_file).unwrap_or(false) {
            self.logger.log(
                &format!("⚠️ Action bindings file not found at {}", bindings_file.display())
            );
            return;
        }

        let content = {
            match fs::read_to_string(&bindings_file) {
                Ok(content) => content,
                Err(e) => {
                    self.logger.log(&format!("❌ Failed to read action bindings file: {e}"));
                    return;
                }
            }
        };

        match self.action_bindings.get_mut(&ty) {
            Some(bindings) => {
                if let Err(e) = bindings.load_json(&content) {
                    self.logger.log(&format!("❌ Failed to load action bindings: {e}"));
                } else {
                    self.logger.log(&format!("✅ Loaded action bindings for {:?}", ty));
                }
            }
            None => {
                self.logger.log(&format!("❌ No action bindings found for {:?}", ty));
            }
        }
    }

    pub fn parse_action_bindings(&mut self, ty: GameInstallType, with_custom: bool) -> bool {
        let game_dir = {
            match self.game_paths.get(&ty) {
                Some(Some(path)) => path,
                _ => {
                    self.logger.log(&format!("❌ No game path found for {:?}", ty));
                    return false;
                }
            }
        };

        let default_bindings_file = self.resource_dir.join("defaultProfile.xml");

        if !fs::exists(&default_bindings_file).unwrap_or(false) {
            self.logger.log(
                &format!(
                    "⚠️ Default bindings file not found at {}",
                    default_bindings_file.display()
                )
            );
            return false;
        }

        let custom_bindings_file = game_dir
            .join("user")
            .join("client")
            .join("0")
            .join("profile")
            .join("default")
            .join("actionmaps.xml");

        if with_custom && !fs::exists(&custom_bindings_file).unwrap_or(false) {
            self.logger.log(
                &format!("⚠️ Custom bindings file not found at {}", custom_bindings_file.display())
            );
        }

        let bindings = {
            match self.action_bindings.get_mut(&ty) {
                Some(bindings) => bindings,
                None => {
                    self.logger.log(&format!("❌ No action bindings found for {:?}", ty));
                    return false;
                }
            }
        };

        self.cached_data_sources
            .get_mut(&ty)
            .map(|ds| ds.take())
            .unwrap_or_default();

        if
            let Err(e) = bindings.load_default_profile(
                &default_bindings_file,
                &SKIP_ACTION_MAPS,
                &ACTION_MAP_UI_CATEGORIES
            )
        {
            self.logger.log(&format!("❌ Failed to load default profile: {e}"));
            return false;
        }

        if with_custom {
            if let Err(e) = bindings.apply_custom_profile(&custom_bindings_file) {
                self.logger.log(&format!("❌ Failed to load custom profile: {e}"));
                return false;
            }
        }

        bindings.generate_missing_binds();

        let bindings_file = {
            match get_appdata_dir() {
                Ok(base) => {
                    let mut path = base;
                    path.push(format!("bindings_{}.json", ty.name()));
                    path
                }
                Err(e) => {
                    self.logger.log(&format!("❌ Failed to get AppData directory: {e}"));
                    return false;
                }
            }
        };

        let json = {
            match bindings.save_json() {
                Ok(json) => json,
                Err(e) => {
                    self.logger.log(&format!("❌ Failed to serialize action bindings: {e}"));
                    return false;
                }
            }
        };

        if let Err(e) = fs::write(&bindings_file, json) {
            self.logger.log(&format!("❌ Failed to write action bindings file: {e}"));
        } else {
            self.logger.log(&format!("✅ Action bindings saved to {}", bindings_file.display()));
        }
        return true;
    }

    pub fn create_profile_xml(
        &self,
        ty: GameInstallType,
        profile_name: &str
    ) -> Result<String, String> {
        let game_dir = {
            match self.game_paths.get(&ty) {
                Some(Some(path)) => path,
                _ => {
                    return Err(format!("❌ No game path found for {:?}", ty));
                }
            }
        };

        let bindings = {
            match self.action_bindings.get(&ty) {
                Some(bindings) => bindings,
                None => {
                    return Err(format!("❌ No action bindings found for {:?}", ty));
                }
            }
        };

        let profile_path = game_dir
            .join("user")
            .join("client")
            .join("0")
            .join("controls")
            .join("mappings")
            .join(format!("{}.xml", PLUGIN_UUID));
        let profile_name_with_timestamp = format!(
            "{}-{}",
            PLUGIN_UUID,
            chrono::Local::now().format("%Y%m%d%H%M%S")
        );

        match bindings.generate_mapping_xml(profile_path, None, &profile_name_with_timestamp) {
            Ok(_) => Ok("Profile XML generated successfully".to_string()),
            Err(e) => Err(format!("❌ Failed to generate mapping XML: {e}")),
        }
    }

    pub fn send_key(&self, action_id: &str) -> Result<(), String> {
        let action_bindings = self.action_bindings
            .get(&GameInstallType::Live)
            .ok_or("No action bindings found for live game")?;

        let action = action_bindings
            .get_binding_by_id(action_id)
            .ok_or_else(|| format!("Action '{}' not found", action_id))?;

        action
            .clone()
            .simulate(self.logger.clone())
            .map_err(|e| format!("Simulation failed: {}", e))
    }
}
