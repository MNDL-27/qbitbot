use std::sync::Arc;

use rutebot::{
    client::Rutebot,
    requests::{ParseMode, SendMessage},
    responses::{Update, User},
};
use tokio::sync::mpsc::{self, Receiver, Sender};

use super::qb_client::QbClient;

pub type RbotParseMode = Option<ParseMode>;

#[derive(Debug)]
struct MessageWrapper {
    pub chat_id: i64,
    pub text: String,
    pub parse_mode: Option<ParseMode>,
}

pub struct QbitBot {
    pub rbot: Arc<Rutebot>,
    qbclient: QbClient,
    tx: Sender<MessageWrapper>,
}

impl QbitBot {
    pub async fn new() -> Self {
        let (tx, rx) = mpsc::channel(32);
        let rbot = Arc::new(Rutebot::new(
            dotenv::var("TOKEN").expect(&format!(dotenv_err!(), "TOKEN")),
        ));
        let qbot = QbitBot {
            qbclient: QbClient::new().await,
            rbot: rbot.clone(),
            tx,
        };
        Self::create_channel_loop(rx, rbot);
        qbot
    }

    pub async fn proccess_message(&self, update: Update) -> Option<()> {
        let message = update.message?;
        let user = message.from?;
        let (response_message, parse_mode) = if self.check_is_admin(&user) {
            self.qbclient.do_cmd(&message.text?).await.ok()?
        } else {
            ("You are not allowed to chat with me".to_string(), None)
        };
        let wrapped_msg = MessageWrapper {
            chat_id: message.chat.id,
            text: response_message,
            parse_mode,
        };
        self.tx.send(wrapped_msg).await.ok()?;
        Some(())
    }

    pub fn check_is_admin(&self, user: &User) -> bool {
        if let Some(username) = &user.username {
            self.qbclient.config.admins.contains(username)
        } else {
            false
        }
    }

    fn create_channel_loop(mut rx: Receiver<MessageWrapper>, rbot: Arc<Rutebot>) {
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                println!("{:#?}", message);
                let reply = SendMessage {
                    parse_mode: message.parse_mode,
                    ..SendMessage::new(message.chat_id, &message.text)
                };
                rbot.prepare_api_request(reply).send().await;
            }
        });
    }
}
