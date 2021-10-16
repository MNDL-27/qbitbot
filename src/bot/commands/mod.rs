use super::qbot::RbotParseMode;

pub mod cmd_list;
pub mod download;
pub mod list;
pub mod pause;
pub mod resume;
pub mod simple;

pub trait QbCommandAction {
    /// Prepare raw Qbittorrent response for Telegram message
    fn action_result_to_string(&self) -> String;
    /// Parse mode (HTML or MD). Default is None
    fn parse_mode(&self) -> RbotParseMode {
        None
    }
}
