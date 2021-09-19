use super::qbot::RbotParseMode;

pub mod cmd_list;
pub mod download;
pub mod list;
pub mod pause;
pub mod resume;
pub mod simple;

pub trait QbCommandAction {
    /// Command name as it should be written in Telegram chat
    fn name(&self) -> String;
    /// Command description to show in /help message
    fn description(&self) -> String {
        "No description".to_string()
    }
    /// Prepare raw Qbittorrent response for Telegram message
    fn action_result_to_string(&self) -> String;
    /// Parse mode (HTML or MD). Default is None
    fn parse_mode(&self) -> RbotParseMode {
        None
    }
}

pub trait BoxedCommand: QbCommandAction + Sized + 'static {
    fn boxed(self) -> Box<dyn QbCommandAction> {
        Box::new(self)
    }
}
