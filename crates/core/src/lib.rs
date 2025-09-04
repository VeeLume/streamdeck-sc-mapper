//! Core library for Star Citizen bindings parsing/generation and install/profile helpers.
//!
//! This crate is UI-agnostic and plugin-agnostic. It exposes:
//! - `bindings`: parse default/custom profiles, generate missing binds, emit XML/JSON.
//! - `sc`: install discovery + profiles I/O helpers + shared enums.
//! - `core_log::CoreLog`: thin logging trait the host (plugin/CLI) can implement.
//!
//! Import the `prelude` if you want the most common types in scope.

pub mod core_log;

pub mod bindings;
pub mod sc;

/// Convenient re-exports for downstream users (plugin/CLI/tests).
pub use core_log::CoreLog;

pub mod prelude {
    pub use crate::core_log::CoreLog;

    // Bindings graph
    pub use crate::bindings::action_binding::ActionBinding;
    pub use crate::bindings::action_bindings::ActionBindings;
    pub use crate::bindings::action_map::ActionMap;
    pub use crate::bindings::activation_mode::{ActivationArena, ActivationMode};
    pub use crate::bindings::bind::{Bind, BindMain, BindOrigin, Key, MouseButton};
    pub use crate::bindings::binds::Binds;
    pub use crate::bindings::binds_generator::BindGenerator;
    pub use crate::bindings::constants::{
        ACTION_MAP_UI_CATEGORIES, CANDIDATE_KEYS, CANDIDATE_MODIFIERS, CATEGORY_GROUPS,
        DEFAULT_CATEGORY, DENY_COMBOS, DENY_MODIFIERS_PER_CATEGORY, SKIP_ACTION_MAPS,
    };

    // Profile I/O helpers
    pub use crate::sc::profiles::{
        appdata_dir, bindings_cache_path, load_bindings_from_appdata, parse_bindings_from_install,
        resolve_custom_profile_from_root, save_bindings_profile_and_cache,
    };

    // Install discovery + enums
    pub use crate::sc::install::{GameInstallType, choose_install_root, scan_paths_and_active};
}
