use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use rutebot::{requests::ParseMode, responses::Update};

use crate::bot::config::QbConfig;
use crate::bot::messages::TelegramBackend;
use crate::bot::qb_chat::QbChat;

use super::qb_client::QbClient;

#[derive(Clone, Debug)]
pub struct MessageWrapper {
    pub text: String,
    pub parse_mode: Option<ParseMode>,
}

pub struct QbitBot {
    rbot: Arc<dyn TelegramBackend>,
    config: QbConfig,
    chats: Arc<RwLock<HashMap<i64, QbChat>>>,
}

impl QbitBot {
    pub async fn new(conf: &QbConfig, rbot: impl TelegramBackend) -> Self {
        QbitBot {
            rbot: Arc::new(rbot),
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

            if chat.select_goto(self.rbot.clone(), &text).await.is_err() {
                info!("Qbit token probably expired. Trying to re-login.");
                chat.relogin()
                    .await
                    .expect("Failed to re-login into Qbittorrent");
                chat.select_goto(self.rbot.clone(), &text)
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
            self.rbot.send_message(chat_id, msg).await;
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
    use crate::bot::config::QbConfig;
    use crate::bot::qbot::QbitBot;
    use crate::bot::rutebot_mock::RutebotMock;
    use rutebot::responses::Update;
    use serde_json::json;

    struct TestCase {
        tg: RutebotMock,
        username: String,
        qbot: QbitBot,
    }

    impl TestCase {
        pub async fn new(by_admin: bool) -> Self {
            let conf = QbConfig::load_path("tests/.env_tests");
            let tg = RutebotMock::default();
            let username = if by_admin {
                conf.admins.iter().next().unwrap()
            } else {
                "BadTester"
            };
            Self {
                qbot: QbitBot::new(&conf, tg.clone()).await,
                username: username.to_owned(),
                tg,
            }
        }

        fn gen_update(&self, text: &str) -> Update {
            let message = json!(
                {
                    "message_id": 0,
                    "date": 0,
                    "from": {"id": 0, "is_bot": false, "first_name": "Test", "username": self.username},
                    "chat": {"id": 0, "type": "private"},
                    "text": text
                }
            );
            serde_json::from_value(json!({"update_id": 0, "message": message})).unwrap()
        }

        async fn send(&self, text: &str) {
            let update = self.gen_update(text);
            self.qbot.process_message(update).await;
        }

        pub fn check(&self, wants: &str) {
            self.tg.assert_last(wants);
        }
    }

    #[tokio::test]
    async fn test_not_admin() {
        let test_case = TestCase::new(false).await;
        test_case.send("Hello, World!").await;
        test_case.check("You are not allowed to chat with me");
    }

    #[tokio::test]
    async fn test_help() {
        let test_case = TestCase::new(true).await;
        test_case.send("/help").await;
        let wants = r#"/download - Start downloading by link or attached file
/help - Show help for all commands
/list - List torrents
/main - Go to main menu

Buttons:

/back"#;
        test_case.check(wants)
    }

    #[tokio::test]
    #[ignore]
    async fn test_download() {
        let test_case = TestCase::new(true).await;
        test_case.send("/download").await;
        let wants = r#"Send torrent link or attach torrent file

Buttons:

/back"#;
        test_case.check(wants);
        let magnet_link = "magnet:?xt=urn:btih:60A2A94625373B5ACAE66D4C693AE5F3417690C1&tr=http%3A%2F%2Fbt3.t-ru.org%2Fann%3Fmagnet&dn=Peter%20Bruce%2C%20Andrew%20Bruce%2C%20Peter%20Gedeck%20%2F%20Питер%20Брюс%2C%20Эндрю%20Брюс%2C%20Питер%20Гедек%20-%20Practical%20Statistics%20for%20Data%20Scientists%20%2F%20Практическая%20статистика%20для";
        test_case.send(magnet_link).await;
        test_case.check("OK");
        test_case.send(magnet_link).await;
        // item is already added
        test_case.check("FAIL");
    }
}
