use std::sync::Arc;
use streamdeck_lib::prelude::*;
use streamdeck_sc_core::CoreLog;

pub struct PluginCoreLog(pub Arc<dyn ActionLog>);

impl CoreLog for PluginCoreLog {
    fn info(&self, msg: &str) {
        self.0.log_level(Level::Info, msg);
    }
    fn warn(&self, msg: &str) {
        self.0.log_level(Level::Warn, msg);
    }
    fn error(&self, msg: &str) {
        self.0.log_level(Level::Error, msg);
    }
    fn debug(&self, msg: &str) {
        self.0.log_level(Level::Debug, msg);
    }
}
