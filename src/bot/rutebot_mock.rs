use async_trait::async_trait;

use crate::bot::qbot::MessageWrapper;
use crate::bot::messages::TelegramBackend;

#[derive(Clone, Default)]
pub struct RutebotMock {
    messages: Vec<String>
}

#[async_trait]
impl TelegramBackend for RutebotMock {
    async fn inner_send_message(&self, chat_id: i64, message: MessageWrapper) {
        info!("Sending message to chat {}: {:#?}", chat_id, message)
    }
}
