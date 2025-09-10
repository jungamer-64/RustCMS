//! Deprecation warning utilities (runtime one-time warnings)
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use tracing::warn;

static TRIGGERED: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();

/// Emit a deprecation warning only once per process for a given tag.
/// `tag` should be a short stable identifier; `msg` the full warning text.
pub fn warn_once(tag: &'static str, msg: &'static str) {
    let set = TRIGGERED.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = match set.lock() {
        Ok(g) => g,
        Err(poison) => poison.into_inner(),
    };
    if guard.insert(tag) {
        warn!(target: "deprecation", "{}", msg);
    }
}
