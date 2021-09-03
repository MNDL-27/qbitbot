use std::{collections::HashMap, sync::Arc};

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
    pub rbot: Arc<Rutebot>,
    qbclient: QbClient,
    chats: HashMap<i64, Sender<MessageWrapper>>,
}

impl QbitBot {
    pub async fn new() -> Self {
        let rbot = Arc::new(Rutebot::new(
            dotenv::var("TOKEN").expect(&format!(dotenv_err!(), "TOKEN")),
        ));
        QbitBot {
            qbclient: QbClient::new().await,
            rbot: rbot.clone(),
            chats: HashMap::new(),
        }
    }

    pub async fn proccess_message(&mut self, update: Update) -> Option<()> {
        let message = update.message?;
        let user = message.from?;
        // TODO: do not create loop for non-admin users
        let tx = self.create_chat_loop(message.chat.id);
        let (response_message, parse_mode) = if self.check_is_admin(&user) {
            self.qbclient
                .do_cmd(&message.text?, tx.clone())
                .await
                .ok()?
        } else {
            ("You are not allowed to chat with me".to_string(), None)
        };
        let wrapped_msg = MessageWrapper {
            text: response_message,
            parse_mode,
        };
        tx.send(wrapped_msg).await.ok()?;
        Some(())
    }

    pub fn check_is_admin(&self, user: &User) -> bool {
        if let Some(username) = &user.username {
            self.qbclient.config.admins.contains(username)
        } else {
            false
        }
    }

    fn create_chat_loop(&mut self, chat_id: i64) -> Sender<MessageWrapper> {
        if let Some(tx) = self.chats.get(&chat_id) {
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
            tx
        }
    }
}
