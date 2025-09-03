//! src/bin/scmap-gen.rs
//! Generate a Star Citizen mappings XML from a game default profile + optional custom profile.
//!
//! Examples:
//!   scmap-gen --default ".\\defaultProfile.xml"
//!   scmap-gen --default ".\\defaultProfile.xml" --out ".\\mappings-generated.xml"
//!   scmap-gen --default ".\\defaultProfile.xml" --install live --profile-name "Veelume Map"
//!   scmap-gen --default ".\\defaultProfile.xml" --bindgen-config ".\\bindgen.json"
//!
//! Notes:
//! - Only --default is required.
//! - Game root is resolved via scan_paths_and_active(); custom profile is
//!   <ROOT>/user/client/0/Profiles/default/actionmaps.xml (with upper/lower fallback).

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use clap::Parser;
use serde::Deserialize;

use streamdeck_lib::prelude::*;

use streamdeck_sc_mapper::bindings::{
    action_bindings::ActionBindings, bind::Bind, binds_generator::BindGenerator, constants::*,
};
use streamdeck_sc_mapper::sc::adapters::install_scanner::scan_paths_and_active;
use streamdeck_sc_mapper::sc::shared::GameInstallType;

fn parse_install_arg(s: &str) -> Result<GameInstallType, String> {
    match s.to_ascii_lowercase().as_str() {
        "live" => Ok(GameInstallType::Live),
        "ptu" => Ok(GameInstallType::Ptu),
        "tech" | "techpreview" | "tp" => Ok(GameInstallType::TechPreview),
        _ => Err("expected one of: live, ptu, tech".into()),
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "scmap-gen",
    version,
    about = "Generate Star Citizen mappings XML"
)]
struct Args {
    /// Path to the default profile XML (game "defaultProfile.xml") [REQUIRED]
    #[arg(short, long)]
    default: PathBuf,

    /// Output path for the resulting mappings XML (defaults next to custom, else next to default)
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Include custom profile (if found/resolved)
    #[arg(short, long)]
    include_custom: bool,

    /// Custom profile to use (defaults to resolved from install root, if any)
    #[arg(long, short)]
    custom: Option<PathBuf>,

    /// Which install to prefer for resolving the custom profile: live | ptu | tech
    #[arg(long, default_value = "live", value_parser = parse_install_arg)]
    install: GameInstallType,

    /// Profile label used in <CustomisationUIHeader label="">
    #[arg(long)]
    profile_name: Option<String>,

    /// Optional JSON to override bind-generation pools/rules (see schema below)
    #[arg(long, value_name = "PATH")]
    bindgen_config: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Print all valid key tokens and exit
    #[arg(long)]
    list_keys: bool,
}

// ───────────────────────────── Logger ─────────────────────────────

#[derive(Clone)]
struct StderrLogger {
    verbose: bool,
}
impl ActionLog for StderrLogger {
    fn log(&self, msg: &str) {
        println!("{msg}");
    }
    fn log_level(&self, level: Level, message: &str) {
        match level {
            Level::Error => eprintln!("ERROR: {message}"),
            Level::Warn => eprintln!("WARN:  {message}"),
            Level::Info => println!("INFO:  {message}"),
            Level::Debug => {
                if self.verbose {
                    println!("DEBUG: {message}")
                }
            }
        }
    }
}

// ───────────────────────────── BindGen config ─────────────────────────────

#[derive(Debug, Deserialize)]
struct BindGenConfig {
    /// e.g. ["f1","f2","np_1","u","i","arrowup", ...]
    candidate_keys: Option<Vec<String>>,
    /// e.g. ["lshift","rshift","lctrl","lalt"]
    candidate_modifiers: Option<Vec<String>>,
    /// e.g. ["lalt+f4","lalt+f9","lalt+lshift+f10"]
    deny_combos: Option<Vec<String>>,
    /// Map of category -> disallowed modifiers, e.g. { "@ui_CCFPS": ["lctrl","lalt","lshift"] }
    disallowed_modifiers_per_category: Option<std::collections::HashMap<String, Vec<String>>>,
}

// ───────────────────────────── main ─────────────────────────────

fn main() -> Result<(), String> {
    let args = Args::parse();

    if args.list_keys {
        // List all valid key tokens, join and separate at 80 char width
        println!("Valid key tokens:");
        let mut line = String::new();
        for k in Key::iter_tokens() {
            if !line.is_empty() {
                if line.len() + 1 + k.len() > 80 {
                    println!("{}", line);
                    line.clear();
                } else {
                    line.push(' ');
                }
            }
            line.push_str(k);
        }
        if !line.is_empty() {
            println!("{}", line);
        }
        println!("Valid modifier tokens:");
        line.clear();
        let mods = [
            "lshift", "rshift", "lctrl", "rctrl", "lalt", "ralt", "lmeta", "rmeta", "lwin", "rwin",
        ];
        // validate all mods parse
        for m in mods {
            if Key::parse(m).is_none() {
                return Err(format!(
                    "internal error: modifier token failed to parse: {m}"
                ));
            }
        }

        for k in mods {
            if !line.is_empty() {
                if line.len() + 1 + k.len() > 80 {
                    println!("{}", line);
                    line.clear();
                } else {
                    line.push(' ');
                }
            }
            line.push_str(k);
        }
        if !line.is_empty() {
            println!("{}", line);
        }
        return Ok(());
    }

    let logger: Arc<dyn ActionLog> = Arc::new(StderrLogger {
        verbose: args.verbose,
    });

    // Validate default profile
    if !args.default.is_file() {
        return Err(format!(
            "--default not found or not a file: {}",
            args.default.display()
        ));
    }
    let default_xml = args.default.clone();
    info!(logger, "Default profile: {}", default_xml.display());

    // if either custom or out isnt given, we need to resolve game root
    let game_root_needed = args.custom.is_none() || args.out.is_none();
    let game_root = if game_root_needed {
        match scan_paths_and_active() {
            Ok((map, last_active)) => choose_install_root(&map, args.install, last_active),
            Err(e) => {
                warn!(logger, "scan_paths_and_active failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Resolve custom profile path if not explicitly given
    let custom_xml = if let Some(c) = args.custom.as_ref() {
        Some(c.clone())
    } else if let Some(root) = game_root.as_ref() {
        let p = resolve_custom_from_root(root.clone());
        if let Some(ref pp) = p {
            info!(logger, "Resolved custom profile: {}", pp.display());
        } else {
            warn!(
                logger,
                "Could not resolve custom profile from install root: {}",
                root.display()
            );
        }
        p
    } else {
        None
    };
    if custom_xml.is_none() {
        warn!(logger, "No custom profile will be applied");
    }

    // Resolve output path if not explicitly given
    let out_path = if let Some(o) = args.out.as_ref() {
        o.clone()
    } else if let Some(root) = game_root.as_ref() {
        let profile_dir = root
            .join("user")
            .join("client")
            .join("0")
            .join("controls")
            .join("mappings");
        derive_output_path(&profile_dir)
    } else {
        derive_output_path(&args.default)
    };

    // Load/merge
    let mut ab = ActionBindings::default();
    ab.load_default_profile(
        &default_xml,
        &*SKIP_ACTION_MAPS,
        &*ACTION_MAP_UI_CATEGORIES,
        &logger,
    )?;

    if args.include_custom {
        if let Some(cf) = custom_xml.as_ref() {
            if cf.try_exists().unwrap_or(false) {
                if let Err(e) = ab.apply_custom_profile(cf, &logger) {
                    warn!(
                        logger,
                        "apply_custom_profile({}): {}",
                        args.install.name(),
                        e
                    );
                }
            } else {
                warn!(logger, "custom file missing at {}", cf.display());
            }
        }
    }
    ab.activation.rebuild_indexes();

    // Generate missing binds (defaults or JSON overrides)
    if let Some(cfg_path) = args.bindgen_config.as_ref() {
        let cfg_text = fs::read_to_string(cfg_path)
            .map_err(|e| format!("read {}: {e}", cfg_path.display()))?;
        let cfg: BindGenConfig = serde_json::from_str(&cfg_text)
            .map_err(|e| format!("parse {}: {e}", cfg_path.display()))?;
        let mut generator = bindgen_from_config(&cfg, &ab.activation, Arc::clone(&logger));
        generator.generate_missing_binds(&mut ab.action_maps);
    } else {
        ab.generate_missing_binds(&logger);
    }

    // Emit XML (devices default internally to keyboard=1/mouse=1)
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create output dir {}: {e}", parent.display()))?;
    }
    let profile_label = args.profile_name.unwrap_or_else(default_profile_label);
    ab.generate_mapping_xml(&out_path, None, &profile_label)?;
    info!(logger, "✅ Wrote {}", out_path.display());
    Ok(())
}

// ───────────────────────────── helpers ─────────────────────────────

fn default_profile_label() -> String {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("Generated {} (scmap-gen)", ts)
}

fn derive_output_path(anchor: &Path) -> PathBuf {
    if anchor.is_dir() {
        anchor.join("mappings-generated.xml")
    } else {
        anchor
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("mappings-generated.xml")
    }
}

/// Pick an install root using preference → last_active → any available.
fn choose_install_root(
    map: &std::collections::HashMap<GameInstallType, Option<PathBuf>>,
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

/// Prefer <ROOT>/user/client/0/Profiles/default/actionmaps.xml
fn resolve_custom_from_root(root: PathBuf) -> Option<PathBuf> {
    let lower = root
        .join("user")
        .join("client")
        .join("0")
        .join("Profiles")
        .join("default")
        .join("actionmaps.xml");
    if lower.is_file() {
        return Some(lower);
    }
    None
}

fn parse_key_list(list: &[String], logger: &Arc<dyn ActionLog>) -> std::collections::HashSet<Key> {
    let mut out = std::collections::HashSet::new();
    for s in list {
        if let Some(k) = Key::parse(s) {
            out.insert(k);
        } else {
            warn!(logger, "bindgen-config: unknown key '{s}' (skipped)");
        }
    }
    out
}

fn parse_binds(list: &[String], logger: &Arc<dyn ActionLog>) -> std::collections::HashSet<Bind> {
    let mut out = std::collections::HashSet::new();
    for s in list {
        match Bind::from_string(s, None) {
            Ok(b) => {
                out.insert(b);
            }
            Err(_) => warn!(logger, "bindgen-config: bad bind '{s}' (skipped)"),
        }
    }
    out
}

fn parse_disallowed_map(
    m: &std::collections::HashMap<String, Vec<String>>,
    logger: &Arc<dyn ActionLog>,
) -> std::collections::HashMap<String, std::collections::HashSet<Key>> {
    let mut out = std::collections::HashMap::new();
    for (cat, keys) in m {
        let set = parse_key_list(keys, logger);
        out.insert(cat.clone(), set);
    }
    out
}

fn bindgen_from_config(
    cfg: &BindGenConfig,
    modes: &streamdeck_sc_mapper::bindings::activation_mode::ActivationArena,
    logger: Arc<dyn ActionLog>,
) -> BindGenerator {
    let available_keys = cfg
        .candidate_keys
        .as_ref()
        .map(|v| parse_key_list(v, &logger))
        .unwrap_or_else(|| CANDIDATE_KEYS.clone());

    let available_modifiers = cfg
        .candidate_modifiers
        .as_ref()
        .map(|v| parse_key_list(v, &logger))
        .unwrap_or_else(|| CANDIDATE_MODIFIERS.clone());

    let banned_binds = cfg
        .deny_combos
        .as_ref()
        .map(|v| parse_binds(v, &logger))
        .unwrap_or_else(|| DENY_COMBOS.clone());

    let disallowed_modifiers = cfg
        .disallowed_modifiers_per_category
        .as_ref()
        .map(|m| parse_disallowed_map(m, &logger))
        .unwrap_or_else(|| {
            DISSALOWED_MODIFIERS_PER_CATEGORY
                .iter()
                .map(|(cat, strs)| {
                    let set = strs
                        .iter()
                        .filter_map(|s| Key::parse(s))
                        .collect::<std::collections::HashSet<_>>();
                    (cat.clone(), set)
                })
                .collect()
        });

    let group_map = CATEGORY_GROUPS
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect::<std::collections::HashMap<_, std::collections::HashSet<_>>>();

    BindGenerator::new(
        modes,
        available_keys,
        available_modifiers,
        banned_binds,
        group_map,
        disallowed_modifiers,
        logger,
    )
}
