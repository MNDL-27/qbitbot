use rutebot::responses::Update;

use qbitbot::bot::config::QbConfig;
use qbitbot::bot::qbot::QbitBot;
use qbitbot::bot::rutebot_mock::RutebotMock;
use serde_json::json;
use std::process::Command;
use std::sync::Arc;

pub const MAGNET_LINK: &str = "magnet:?xt=urn:btih:60A2A94625373B5ACAE66D4C693AE5F3417690C1&tr=http%3A%2F%2Fbt3.t-ru.org%2Fann%3Fmagnet&dn=Peter%20Bruce%2C%20Andrew%20Bruce%2C%20Peter%20Gedeck%20%2F%20Питер%20Брюс%2C%20Эндрю%20Брюс%2C%20Питер%20Гедек%20-%20Practical%20Statistics%20for%20Data%20Scientists%20%2F%20Практическая%20статистика%20для";

pub struct TestCase {
    tg: RutebotMock,
    admin: String,
    qbot: QbitBot,
}

impl TestCase {
    pub async fn new() -> Self {
        let mut handle = Command::new("./start.sh")
            .current_dir("tests/docker_qb")
            .spawn()
            .expect("Failed to start Qbittorrent docker");
        handle.wait().unwrap();
        let conf = QbConfig::load_path("tests/.env_tests");
        let admin = conf.admins.iter().next().unwrap().to_owned();
        let tg = RutebotMock::default();
        Self {
            qbot: QbitBot::new(&conf, tg.clone()).await,
            admin,
            tg,
        }
    }

    fn gen_update(&self, text: &str, username: &str) -> Update {
        let message = json!(
            {
                "message_id": 0,
                "date": 0,
                "from": {"id": 0, "is_bot": false, "first_name": "Test", "username": username},
                "chat": {"id": 0, "type": "private"},
                "text": text
            }
        );
        serde_json::from_value(json!({"update_id": 0, "message": message})).unwrap()
    }

    pub async fn send(&self, text: &str) {
        let update = self.gen_update(text, &self.admin);
        self.qbot.process_message(update).await;
    }

    pub async fn send_non_admin(&self, text: &str, username: &str) {
        let update = self.gen_update(text, username);
        self.qbot.process_message(update).await;
    }

    pub fn check(&self, wants: &str) {
        self.tg.assert_last(wants);
    }

    pub fn get_tg_arc(&self) -> Arc<RutebotMock> {
        Arc::new(self.tg.clone())
    }
}

impl Drop for TestCase {
    fn drop(&mut self) {
        let mut handle = Command::new("./stop.sh")
            .current_dir("tests/docker_qb")
            .spawn()
            .expect("Failed to stop Qbittorrent docker");
        handle.wait().unwrap();
    }
}
