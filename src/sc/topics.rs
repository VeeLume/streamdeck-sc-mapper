use streamdeck_lib::prelude::*;

use crate::sc::shared::GameInstallType;

pub const EXEC_SEND: TopicId<ExecSend> = TopicId::new("sc.exec.send");

#[derive(Debug, Clone)]
pub struct ExecSend {
    pub action_id: String,
    pub hold_ms: Option<u64>,
    pub is_down: Option<bool>,
}

pub const INSTALL_SCAN: TopicId<()> = TopicId::new("sc.install.scan");
pub const INITIAL_INSTALL_SCAN_DONE: TopicId<()> = TopicId::new("sc.install.initial-scan-done");
pub const INSTALL_UPDATED: TopicId<()> = TopicId::new("sc.install.updated");
pub const INSTALL_ACTIVE_CHANGED: TopicId<InstallActiveChanged> =
    TopicId::new("sc.install.active-changed");
pub struct InstallActiveChanged {
    pub ty: GameInstallType, // "LIVE" | "PTU" | "TechPreview"
}

pub const BINDINGS_PARSED: TopicId<()> = TopicId::new("sc.bindings.parsed");
pub const BINDINGS_REBUILD_AND_SAVE: TopicId<BindingsRebuildAndSave> =
    TopicId::new("sc.bindings.rebuild-and-save");
pub struct BindingsRebuildAndSave {
    pub ty: GameInstallType,  // "LIVE" | "PTU" | "TechPreview"
    pub with_custom: bool,    // true for custom bindings, false for default
    pub name: Option<String>, // Optional profile name
}

pub const ACTIONS_REQUEST: TopicId<()> = TopicId::new("sc.actions.request");
pub const ACTIONS_CACHE_UPDATED: TopicId<()> = TopicId::new("sc.actions.cache-updated");
