use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use rutebot::{client::Rutebot, requests::ParseMode, responses::Update};

use crate::bot::config::QbConfig;
use crate::bot::qb_chat::QbChat;

use super::qb_client::QbClient;

#[derive(Debug)]
pub struct MessageWrapper {
    pub text: String,
    pub parse_mode: Option<ParseMode>,
}

pub struct QbitBot {
    pub rbot: Rutebot,
    config: QbConfig,
    chats: Arc<RwLock<HashMap<i64, QbChat>>>,
}

impl QbitBot {
    pub async fn new(config: QbConfig) -> Self {
        let rbot = Rutebot::new(config.token.to_owned());
        QbitBot {
            config,
            rbot,
            chats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn process_message(self: Arc<Self>, update: Update) -> Option<()> {
        let message = update.message?;
        let text = message.text?;
        let chat_id = message.chat.id;
        let username = message.from?.username?;
        let is_admin = self.config.admins.contains(&username);
        if is_admin {
            let mut lock = self.chats.write().unwrap();
            // TODO: do not create new client every time
            let qbclient = QbClient::new(&self.config).await;
            let chat = lock
                .entry(chat_id)
                .or_insert_with(|| QbChat::new(chat_id, self.rbot.clone(), qbclient));
            if chat.select_goto(&text).await.is_err() {
                info!("Qbit token probably expired. Trying to re-login.");
                chat.relogin()
                    .await
                    .expect("Failed to re-login into Qbittorrent");
                chat.select_goto(&text).await.unwrap_or_else(|_| {
                    error!("There is an error after re-login. Probably something has broken")
                });
            };
            Some(())
        } else {
            info!(
                "User {} tried to chat with qbot but he does not have access",
                username
            );
            None
        }
    }
}
