use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use rutebot::{
    client::Rutebot,
    requests::{ParseMode, SendMessage},
    responses::{Update, User},
};
use tokio::sync::mpsc::{self, Receiver, Sender};

use super::qb_client::QbClient;
use crate::bot::qb_chat::QbChat;

pub type RbotParseMode = Option<ParseMode>;

#[derive(Debug)]
pub struct MessageWrapper {
    pub text: String,
    pub parse_mode: Option<ParseMode>,
}

pub struct QbitBot {
    pub rbot: Rutebot,
    qbclient: QbClient,
    chats: RwLock<HashMap<i64, QbChat>>,
}

impl QbitBot {
    pub async fn new() -> Self {
        let rbot = Rutebot::new(dotenv::var("TOKEN").expect(&format!(dotenv_err!(), "TOKEN")));
        QbitBot {
            qbclient: QbClient::new().await,
            rbot,
            chats: RwLock::new(HashMap::new()),
        }
    }

    pub async fn process_message(self: Arc<Self>, update: Update) -> Option<()> {
        let message = update.message?;
        let text = message.text?;
        debug!("Checking whether chat_id is presented or not");
        let maybe_chat = {
            let lock = self.chats.read().unwrap();
            lock.get(&message.chat.id).is_some()
        };
        if maybe_chat {
            let mut lock = self.chats.write().unwrap();
            let chat = lock.get_mut(&message.chat.id).unwrap();
            chat.select_goto(&text).await;
        } else {
            debug!("Created new chat");
            let mut chat = QbChat::new(message.chat.id, self.rbot.clone());
            let mut lock = self.chats.write().unwrap();
            lock.insert(message.chat.id, chat.clone());
            chat.select_goto(&text).await;
        };
        Some(())
    }
}
