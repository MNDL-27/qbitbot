use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::{qb_chat::QbChat, qbot::MessageWrapper};

pub enum CheckType {
    /// Completed(name) check that torrent named 'name' is completed
    Completed(String),
}

pub trait Notifier {
    fn get_tx(chat: QbChat) -> Sender<CheckType> {
        let (tx, rx) = channel(8);
        Self::create_notify_loop(rx, chat);
        tx
    }

    fn create_notify_loop(mut rx: Receiver<CheckType>, chat: QbChat) {
        tokio::spawn(async move {
            while let Some(check) = rx.recv().await {
                let send_text = match check {
                    CheckType::Completed(name) => {
                        format!("{} is done", name)
                    }
                };
                let message = MessageWrapper {
                    text: send_text,
                    parse_mode: None,
                };
                chat.chat_send_message(message).await
            }
        });
    }
}
