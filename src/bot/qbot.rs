use rutebot::{
    client::{ApiRequest, Rutebot},
    requests::SendMessage,
    responses::{Message, Update, User},
};

use super::qb_client::QbClient;

pub struct QbitBot {
    pub rbot: Rutebot,
    qbclient: QbClient,
}

impl QbitBot {
    pub async fn new() -> Self {
        QbitBot {
            rbot: Rutebot::new(dotenv::var("TOKEN").expect(&format!(dotenv_err!(), "TOKEN"))),
            qbclient: QbClient::new().await,
        }
    }

    pub async fn proccess_message(&self, update: Update) -> Option<ApiRequest<Message>> {
        let message = update.message?;
        let user = message.from?;
        let response_message = if self.check_is_admin(&user) {
            self.qbclient.do_cmd(&message.text?).await.ok()?
        } else {
            "You are not allowed to chat with me".to_string()
        };
        let reply = SendMessage::new(message.chat.id, &response_message);
        Some(self.rbot.prepare_api_request(reply))
    }

    pub fn check_is_admin(&self, user: &User) -> bool {
        if let Some(username) = &user.username {
            self.qbclient.config.admins.contains(username)
        } else {
            false
        }
    }
}
