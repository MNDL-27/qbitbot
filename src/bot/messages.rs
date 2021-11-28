use std::time::Duration;

use async_trait::async_trait;
use fure::backoff::fixed;
use fure::policies::{attempts, backoff};
use rutebot::client::Rutebot;
use rutebot::requests::SendMessage;

use crate::bot::qbot::MessageWrapper;

#[async_trait]
pub trait TelegramBackend: Sync + Send + 'static {
    async fn send_message(&self, chat_id: i64, message: MessageWrapper);
}

#[async_trait]
impl TelegramBackend for Rutebot {
    async fn send_message(&self, chat_id: i64, message: MessageWrapper) {
        debug!("Sending message to chat({}): {:#?}", chat_id, message);
        let send_msg = || async {
            let reply = SendMessage {
                parse_mode: message.parse_mode,
                ..SendMessage::new(chat_id, &message.text)
            };
            self.prepare_api_request(reply).send().await
        };
        let policy = attempts(backoff(fixed(Duration::from_secs(3))), 3);
        if fure::retry(send_msg, policy).await.is_err() {
            error!("Failed to send reply 4 times")
        };
    }
}