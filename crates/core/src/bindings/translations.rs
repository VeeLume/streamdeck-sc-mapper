use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{CoreLog, bindings::action_bindings::ActionBindings};

/// Public entry: returns a tiny map only for keys used by ActionBindings.
/// Reuses cache if (a) keys hash matches and (b) cache is newer than global.ini.
pub fn load_translations_cached_from_bindings(
    global_ini_path: PathBuf,
    bindings: &ActionBindings,
    cache_path: PathBuf,
    logger: &Arc<dyn CoreLog>,
) -> HashMap<String, String> {
    let used = collect_translation_keys_from_bindings(bindings);

    // Cache hot path
    if let Some(map) = try_load_cache_if_fresh(&cache_path, &global_ini_path, &used) {
        return map;
    }

    // Cold path: build subset from the large file
    let bytes = match fs::read(&global_ini_path) {
        Ok(b) => b,
        Err(e) => {
            logger.warn(&format!("read {}: {}", global_ini_path.display(), e));
            return HashMap::new();
        }
    };
    let mut content = decode_text(&bytes, &global_ini_path, logger);

    // Defensive cleanup
    if content.starts_with('\u{feff}') {
        content.remove(0);
    }
    if content.contains('\0') {
        logger.warn(&format!(
            "decode {}: NULs found; stripping",
            global_ini_path.display()
        ));
        content.retain(|c| c != '\0');
    }

    // Filter only needed keys; stop early if we have them all
    let target = used.len();
    let mut map = HashMap::with_capacity(target);
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = parse_line(t) {
            if used.contains(k) || used.contains(&format!("@{k}")) {
                map.insert(k.to_string(), v.to_string());
                if map.len() >= target {
                    break;
                }
            }
        }
    }

    // Persist cache
    let cache = CacheFile {
        keys_hash: hash_keys(&used),
        map: &map,
    };
    if let Ok(bytes) = serde_json::to_vec(&cache) {
        if let Err(e) = fs::write(&cache_path, bytes) {
            logger.warn(&format!("write cache {}: {}", cache_path.display(), e));
        }
    }

    map
}

/// Pull all translation tokens referenced by bindings.
/// Accepts both direct fields and any text that may contain multiple @tokens.
fn collect_translation_keys_from_bindings(bindings: &ActionBindings) -> HashSet<String> {
    let mut out = HashSet::<String>::new();

    // Action map-level labels/categories
    for (_k, am) in &bindings.action_maps {
        maybe_collect_tokens(am.ui_label.as_deref(), &mut out);
        maybe_collect_tokens(am.ui_category.as_deref(), &mut out);

        for (_ak, ab) in &am.actions {
            maybe_collect_tokens(ab.ui_label.as_deref(), &mut out);
            maybe_collect_tokens(ab.ui_description.as_deref(), &mut out);
            maybe_collect_tokens(ab.category.as_deref(), &mut out);
        }
    }

    // Also keep bare keys (without '@') so we match INI keys with/without '@'
    let mut bare = Vec::with_capacity(out.len());
    for k in &out {
        if let Some(stripped) = k.strip_prefix('@') {
            bare.push(stripped.to_string());
        }
    }
    out.extend(bare);

    out
}

/// Scan a string and collect all @Tokens (A-Za-z0-9_.-).
fn maybe_collect_tokens(text: Option<&str>, out: &mut HashSet<String>) {
    let Some(s) = text else { return };
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                let c = bytes[i];
                let ok = c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b'-';
                if ok {
                    i += 1;
                } else {
                    break;
                }
            }
            if i > start + 1 {
                out.insert(s[start..i].to_string());
                continue;
            }
        }
        i += 1;
    }
}

// Borrowing version: only for WRITING the cache (no Deserialize here)
#[derive(Serialize)]
struct CacheFile<'a> {
    keys_hash: u64,
    #[serde(borrow)]
    map: &'a HashMap<String, String>,
}

// Owned version: used when READING the cache from disk
#[derive(Serialize, Deserialize)]
struct CacheFileOwned {
    keys_hash: u64,
    map: HashMap<String, String>,
}

fn try_load_cache_if_fresh(
    cache_path: &Path,
    global_ini_path: &Path,
    used: &HashSet<String>,
) -> Option<HashMap<String, String>> {
    let cache_meta = fs::metadata(cache_path).ok()?;
    let cache_mtime = cache_meta.modified().ok()?;
    let ini_mtime = fs::metadata(global_ini_path).ok()?.modified().ok()?;
    if cache_mtime < ini_mtime {
        return None; // source newer → rebuild
    }

    let bytes = fs::read(cache_path).ok()?;
    let parsed: CacheFileOwned = serde_json::from_slice(&bytes).ok()?;
    if parsed.keys_hash != hash_keys(used) {
        return None; // bindings changed → rebuild
    }

    Some(parsed.map)
}

fn hash_keys(used: &HashSet<String>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut v: Vec<&String> = used.iter().collect();
    v.sort_unstable_by(|a, b| a.as_str().cmp(b.as_str())); // order-independent hash
    let mut h = DefaultHasher::new();
    for k in v {
        k.hash(&mut h);
    }
    h.finish()
}

/// Tolerant decoder with BOM detection (UTF-8/UTF-16LE/BE) and lossy fallback.
fn decode_text(bytes: &[u8], path: &Path, logger: &Arc<dyn CoreLog>) -> String {
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        match std::str::from_utf8(&bytes[3..]) {
            Ok(s) => s.to_owned(),
            Err(e) => {
                logger.warn(&format!(
                    "decode {} (utf-8+bom): {}; using lossy",
                    path.display(),
                    e
                ));
                String::from_utf8_lossy(&bytes[3..]).into_owned()
            }
        }
    } else if bytes.starts_with(b"\xFF\xFE") {
        let data = &bytes[2..];
        let mut u16s = Vec::with_capacity(data.len() / 2);
        let mut chunks = data.chunks_exact(2);
        for ch in &mut chunks {
            u16s.push(u16::from_le_bytes([ch[0], ch[1]]));
        }
        String::from_utf16_lossy(&u16s)
    } else if bytes.starts_with(b"\xFE\xFF") {
        let data = &bytes[2..];
        let mut u16s = Vec::with_capacity(data.len() / 2);
        let mut chunks = data.chunks_exact(2);
        for ch in &mut chunks {
            u16s.push(u16::from_be_bytes([ch[0], ch[1]]));
        }
        String::from_utf16_lossy(&u16s)
    } else {
        match std::str::from_utf8(bytes) {
            Ok(s) => s.to_owned(),
            Err(_) => {
                logger.warn(&format!(
                    "decode {}: not UTF-8; using lossy fallback",
                    path.display()
                ));
                match String::from_utf8_lossy(bytes) {
                    Cow::Borrowed(s) => s.to_owned(),
                    Cow::Owned(s) => s,
                }
            }
        }
    }
}

/// Same parse logic you had before.
fn parse_line(line: &str) -> Option<(&str, &str)> {
    if let Some(i) = line.find(",P=") {
        let (k, v) = line.split_at(i);
        return Some((k.trim(), v.trim_start_matches(",P=").trim()));
    }
    if let Some(i) = line.find(',') {
        let (k, v) = line.split_at(i);
        return Some((k.trim(), v.trim_start_matches(',').trim()));
    }
    if let Some(i) = line.find('=') {
        let (k, v) = line.split_at(i);
        return Some((k.trim(), v.trim_start_matches('=').trim()));
    }
    None
}

pub(crate) fn get_translation<'a>(
    key: &'a str,
    translations: &'a std::collections::HashMap<String, String>,
) -> &'a str {
    let clean_key = key.strip_prefix('@').unwrap_or(key);
    translations
        .get(clean_key)
        .map(String::as_str)
        .unwrap_or(key)
}
