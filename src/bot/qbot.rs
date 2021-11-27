use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use rutebot::{client::Rutebot, requests::ParseMode, responses::Update};

use crate::bot::config::QbConfig;
use crate::bot::messages::send_message;
use crate::bot::qb_chat::QbChat;

use super::qb_client::QbClient;

#[derive(Debug)]
pub struct MessageWrapper {
    pub text: String,
    pub parse_mode: Option<ParseMode>,
}

pub struct QbitBot {
    rbot: Rutebot,
    config: QbConfig,
    chats: Arc<RwLock<HashMap<i64, QbChat>>>,
}

impl QbitBot {
    pub async fn new(conf: &QbConfig, rbot: Rutebot) -> Self {
        QbitBot {
            rbot,
            config: conf.to_owned(),
            chats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn process_message(&self, update: Update) -> Option<()> {
        let message = update.message?;
        let text = message.text?;
        let chat_id = message.chat.id;
        let username = message.from?.username?;
        let is_admin = self.config.admins.contains(&username);
        if is_admin {
            let mut chat = if let Some(chat) = self.chats.read().unwrap().get(&chat_id) {
                chat.to_owned()
            } else {
                let qbclient = QbClient::new(&self.config).await;
                QbChat::new(chat_id, qbclient)
            };

            if chat.select_goto(self.rbot.to_owned(), &text).await.is_err() {
                info!("Qbit token probably expired. Trying to re-login.");
                chat.relogin()
                    .await
                    .expect("Failed to re-login into Qbittorrent");
                chat.select_goto(self.rbot.to_owned(), &text)
                    .await
                    .unwrap_or_else(|_| {
                        error!("There is an error after re-login. Probably something has broken")
                    });
            };

            self.chats.write().unwrap().insert(chat_id, chat);

            Some(())
        } else {
            let msg = MessageWrapper {
                text: String::from("You are not allowed to chat with me"),
                parse_mode: None,
            };
            send_message(self.rbot.to_owned(), chat_id, msg).await;
            info!(
                "User {} tried to chat with qbot but he does not have access",
                username
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use rutebot::responses::Update;

    fn gen_message(chat_id: i64, username: String, text: String) -> Update {
        Update {
            update_id: 0,
            message: None,
            edited_message: None,
            channel_post: None,
            callback_query: None,
            edited_channel_post: None,
        }
    }

    #[tokio::test]
    async fn test_not_admin() {}
}
