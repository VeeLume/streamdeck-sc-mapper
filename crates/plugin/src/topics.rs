use streamdeck_lib::prelude::*;
use streamdeck_sc_core::prelude::GameInstallType;

// Execute an action by ID (used by ExecAdapter)
pub const EXEC_SEND: TopicId<ExecSend> = TopicId::new("sc.exec.send");

#[derive(Debug, Clone)]
pub struct ExecSend {
    pub action_id: String,
    pub hold_ms: Option<u64>,
    pub is_down: Option<bool>,
}

// Install scanning + active-channel tracking
pub const INSTALL_SCAN: TopicId<()> = TopicId::new("sc.install.scan");
pub const INITIAL_INSTALL_SCAN_DONE: TopicId<()> = TopicId::new("sc.install.initial-scan-done");
pub const INSTALL_UPDATED: TopicId<()> = TopicId::new("sc.install.updated");

pub const INSTALL_ACTIVE_CHANGED: TopicId<InstallActiveChanged> =
    TopicId::new("sc.install.active-changed");

#[derive(Debug, Clone)]
pub struct InstallActiveChanged {
    /// LIVE | PTU | TechPreview
    pub ty: GameInstallType,
}

// Bindings lifecycle
pub const BINDINGS_PARSED: TopicId<()> = TopicId::new("sc.bindings.parsed");

pub const BINDINGS_REBUILD_AND_SAVE: TopicId<BindingsRebuildAndSave> =
    TopicId::new("sc.bindings.rebuild-and-save");

#[derive(Debug, Clone)]
pub struct BindingsRebuildAndSave {
    /// LIVE | PTU | TechPreview
    pub ty: GameInstallType,
    /// true to include custom profile overlay
    pub with_custom: bool,
    /// Optional profile label for <CustomisationUIHeader label="">
    pub name: Option<String>,
}

// UI/data cache triggers
pub const ACTIONS_REQUEST: TopicId<()> = TopicId::new("sc.actions.request");
pub const ACTIONS_CACHE_UPDATED: TopicId<()> = TopicId::new("sc.actions.cache-updated");
