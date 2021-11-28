use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;

use crate::bot::messages::TelegramBackend;

use super::qbot::MessageWrapper;
use std::sync::Arc;

pub enum CheckType {
    /// Completed(name) check that torrent named 'name' is completed
    Completed(String),
}

pub trait Notifier {
    fn create_notifier_tx(rbot: Arc<dyn TelegramBackend>, chat_id: i64) -> Sender<CheckType> {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            if let Ok(check) = rx.await {
                let send_text = match check {
                    CheckType::Completed(name) => {
                        format!("{} is done", name)
                    }
                };
                let message = MessageWrapper {
                    text: send_text,
                    parse_mode: None,
                };
                rbot.send_message(chat_id, message).await;
            }
        });
        tx
    }
}
