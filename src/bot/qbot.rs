use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use rutebot::{client::Rutebot, requests::ParseMode, responses::Update};

use crate::bot::qb_chat::QbChat;

use super::qb_client::QbClient;

pub type RbotParseMode = Option<ParseMode>;

#[derive(Debug)]
pub struct MessageWrapper {
    pub text: String,
    pub parse_mode: Option<ParseMode>,
}

pub struct QbitBot {
    pub rbot: Rutebot,
    qbclient: Arc<QbClient>,
    chats: Arc<RwLock<HashMap<i64, QbChat>>>,
}

impl QbitBot {
    pub async fn new() -> Self {
        let rbot = Rutebot::new(dotenv::var("TOKEN").expect(&format!(dotenv_err!(), "TOKEN")));
        QbitBot {
            qbclient: Arc::new(QbClient::new().await),
            rbot,
            chats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn process_message(self: Arc<Self>, update: Update) -> Option<()> {
        let message = update.message?;
        let text = message.text?;
        let chat_id = message.chat.id;
        let mut lock = self.chats.write().unwrap();
        let chat = lock
            .entry(chat_id)
            .or_insert_with(|| QbChat::new(chat_id, self.rbot.clone(), self.qbclient.clone()));
        chat.select_goto(&text).await;
        Some(())
    }
}
