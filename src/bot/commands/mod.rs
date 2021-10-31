pub mod cmd_list;
pub mod download;
pub mod list;
pub mod pause_resume;
pub mod simple;
pub mod aux;

pub trait QbCommandAction {
    /// Prepare raw Qbittorrent response for Telegram message
    fn action_result_to_string(&self) -> String;
}
