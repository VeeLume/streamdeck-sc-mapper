mod data_source;
mod serde_helpers;
pub mod bindings {
    mod action_binding;
    pub mod action_bindings;
    mod action_map;
    pub mod activation_mode;
    pub mod bind;
    mod bind_tokens;
    mod binds;
    pub mod binds_generator;
    pub mod constants;
    mod generate_mappings_xml;
    mod helpers;
    mod str_intern;
    pub mod translations;
}
pub mod sc {
    pub mod shared;
    pub mod topics;
    pub mod adapters {
        pub mod bindings_adapter;
        pub mod exec_adapter;
        pub mod install_scanner;
    }
}
pub mod actions {
    pub mod generate_profile;
    pub mod rotate_install;
    pub mod sc_action;
}
pub const PLUGIN_ID: &str = "icu.veelume.sc-mapper";
