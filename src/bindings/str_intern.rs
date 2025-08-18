use std::{ sync::Arc, collections::HashMap };
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{ Deserialize, Deserializer, Serializer };

static INTERN: Lazy<RwLock<HashMap<String, Arc<str>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

/// Intern a &str -> Arc<str>. Equal strings share the same Arc buffer.
pub fn intern<S: AsRef<str>>(s: S) -> Arc<str> {
    let s = s.as_ref();
    // fast path: read lock
    if let Some(existing) = INTERN.read().get(s) {
        return Arc::clone(existing);
    }
    // slow path: upgrade to write
    let mut w = INTERN.write();
    if let Some(existing) = w.get(s) {
        return Arc::clone(existing);
    }
    let arc: Arc<str> = Arc::from(s.to_owned());
    // store with an owned key (keeps one String copy in the map)
    w.insert(arc.to_string(), Arc::clone(&arc));
    arc
}

/// Serde helper to (de)serialize Arc<str> as a plain string, interning on read.

// For Arc<str>

pub mod serde_arcstr {
    use super::*;
    pub fn serialize<S: Serializer>(v: &Arc<str>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(v)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Arc<str>, D::Error> {
        let s = String::deserialize(d)?;
        Ok(super::intern(s))
    }

    // --- Option<Arc<str>> helpers ---
    pub mod opt {
        use super::*;
        pub fn serialize<S: Serializer>(v: &Option<Arc<str>>, s: S) -> Result<S::Ok, S::Error> {
            match v {
                Some(v) => s.serialize_some(&v.as_ref()),
                None => s.serialize_none(),
            }
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Arc<str>>, D::Error> {
            let opt = Option::<String>::deserialize(d)?;
            Ok(opt.map(super::intern))
        }
    }
}
