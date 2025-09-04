use core::fmt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, str::FromStr};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
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

impl fmt::Display for GameInstallType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for GameInstallType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "live" => Ok(GameInstallType::Live),
            "ptu" => Ok(GameInstallType::Ptu),
            "tech" | "techpreview" | "tp" => Ok(GameInstallType::TechPreview),
            _ => Err("expected one of: live, ptu, tech"),
        }
    }
}

/// Pure value type for discovered install paths.
pub type InstallPathMap = HashMap<GameInstallType, Option<PathBuf>>;

/// Scan the RSI Launcher log to discover install roots for LIVE/PTU/Tech Preview,
/// and return a map plus the most recently active channel (if seen).
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

/// Pick an install root using preference → last_active → any available.
pub fn choose_install_root(
    map: &HashMap<GameInstallType, Option<PathBuf>>,
    prefer: GameInstallType,
    last_active: Option<GameInstallType>,
) -> Option<PathBuf> {
    if let Some(Some(p)) = map.get(&prefer) {
        return Some(p.clone());
    }
    if let Some(ty) = last_active {
        if let Some(Some(p)) = map.get(&ty) {
            return Some(p.clone());
        }
    }
    for ty in GameInstallType::ALL {
        if let Some(Some(p)) = map.get(&ty) {
            return Some(p.clone());
        }
    }
    None
}
