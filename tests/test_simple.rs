use serde::Serialize;

use common::TestCase;
use qbitbot::bot::config::QbConfig;
use qbitbot::bot::qb_client::QbClient;

pub mod common;

#[tokio::test]
async fn run_simple_tests() {
    let test_case = TestCase::new().await;
    test_not_admin(&test_case).await;
    test_help(&test_case).await;
    test_client_start().await;
}

async fn test_not_admin(test_case: &TestCase) {
    test_case.send_non_admin("Hello, World!", "BadTester").await;
    test_case.check("You are not allowed to chat with me");
}

async fn test_help(test_case: &TestCase) {
    test_case.send("/help").await;
    let wants = r#"/download - Start downloading by link or attached file
/help - Show help for all commands
/list - List torrents
/main - Go to main menu

Buttons:

/back"#;
    test_case.check(wants)
}

#[derive(Serialize)]
struct EmptyAction{
    test: i32
}

async fn test_client_start() {
    let conf = QbConfig::load_path("tests/.env_tests");
    let client = QbClient::new(&conf).await;
    client.qpost("/app/version", EmptyAction{test: 1}).await.unwrap();
}
