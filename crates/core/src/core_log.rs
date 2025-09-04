/// Minimal logger for core so we don't depend on streamdeck-lib.
/// Implement this in the CLI and adapt it from the plugin.
pub trait CoreLog: Send + Sync {
    fn info(&self, msg: &str) {
        let _ = msg;
    }
    fn warn(&self, msg: &str) {
        let _ = msg;
    }
    fn error(&self, msg: &str) {
        let _ = msg;
    }
    fn debug(&self, msg: &str) {
        let _ = msg;
    }
}

/// No-op logger if you don't care about logs.
pub struct NoopLog;
impl CoreLog for NoopLog {}
