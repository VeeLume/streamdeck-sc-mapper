use std::{ sync::Arc, collections::HashMap };
use once_cell::sync::Lazy;
use parking_lot::RwLock;

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
