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

pub type RbotParseMode = Option<ParseMode>;

#[derive(Debug)]
pub struct MessageWrapper {
    pub text: String,
    pub parse_mode: Option<ParseMode>,
}

pub struct QbitBot {
    pub rbot: Rutebot,
    qbclient: QbClient,
    chats: Arc<RwLock<HashMap<i64, Sender<MessageWrapper>>>>,
}

impl QbitBot {
    pub async fn new() -> Self {
        let rbot = Rutebot::new(dotenv::var("TOKEN").expect(&format!(dotenv_err!(), "TOKEN")));
        QbitBot {
            qbclient: QbClient::new().await,
            rbot,
            chats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn chat_work(
        &self,
        text: &str,
        tx: &Sender<MessageWrapper>,
    ) -> Result<(String, RbotParseMode)> {
        self.qbclient.do_cmd(text, tx.clone()).await
    }

    pub async fn proccess_message(self: Arc<Self>, update: Update) -> Option<()> {
        let message = update.message?;
        let user = message.from?;
        let text = message.text?;
        // TODO: do not create loop for non-admin users
        let tx = self.create_chat_loop(message.chat.id);
        tokio::spawn(async move {
            let (response_message, parse_mode) = if self.check_is_admin(&user) {
                let cmd_result = self.chat_work(&text, &tx).await;
                if let Ok(res) = cmd_result {
                    res
                } else {
                    self.qbclient
                        .login()
                        .await
                        .expect("Couldnt re-login into qBittorrent");
                    self.chat_work(&text, &tx)
                        .await
                        .expect("Command execution failed")
                }
            } else {
                ("You are not allowed to chat with me".to_string(), None)
            };
            let wrapped_msg = MessageWrapper {
                text: response_message,
                parse_mode,
            };
            if tx.send(wrapped_msg).await.is_err() {
                println!("Failed to send reply")
            };
        });
        Some(())
    }

    pub fn check_is_admin(&self, user: &User) -> bool {
        if let Some(username) = &user.username {
            self.qbclient.config.admins.contains(username)
        } else {
            false
        }
    }

    fn create_chat_loop(&self, chat_id: i64) -> Sender<MessageWrapper> {
        if let Some(tx) = self.chats.read().unwrap().get(&chat_id) {
            tx.clone()
        } else {
            let (tx, mut rx): (Sender<MessageWrapper>, Receiver<MessageWrapper>) =
                mpsc::channel(32);
            let rbot = self.rbot.clone();
            tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    println!("{:#?}", message);
                    let reply = SendMessage {
                        parse_mode: message.parse_mode,
                        ..SendMessage::new(chat_id, &message.text)
                    };
                    rbot.prepare_api_request(reply).send().await;
                }
            });
            self.chats
                .write()
                .unwrap()
                .extend(vec![(chat_id, tx.clone())]);
            tx
        }
    }
}
