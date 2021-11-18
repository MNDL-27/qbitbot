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
    qbclient: Arc<RwLock<QbClient>>,
    chats: Arc<RwLock<HashMap<i64, QbChat>>>,
}

impl QbitBot {
    pub async fn new(config: &QbConfig) -> Self {
        let rbot = Rutebot::new(config.token.to_owned());
        QbitBot {
            qbclient: Arc::new(RwLock::new(QbClient::new(config).await)),
            rbot,
            chats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn process_message(self: Arc<Self>, update: Update) -> Option<()> {
        let message = update.message?;
        let text = message.text?;
        let chat_id = message.chat.id;
        let username = message.from?.username?;
        // TODO: resolve deadlock
        // let is_admin = self.qbclient.read().ok()?.config.admins.contains(&username);
        if true {
            let mut lock = self.chats.write().unwrap();
            let chat = lock
                .entry(chat_id)
                .or_insert_with(|| QbChat::new(chat_id, self.rbot.clone(), self.qbclient.clone()));
            if chat.select_goto(&text).await.is_err() {
                self.qbclient
                    .write()
                    .unwrap()
                    .login()
                    .await
                    .expect("Failed to re-login into Qbittorrent");
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
