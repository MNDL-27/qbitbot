use std::sync::Arc;

use common::{MAGNET_LINK, RutebotMock, TestCase};
use qbitbot::bot::config::QbConfig;
use qbitbot::bot::qb_chat::MenuValue::*;
use qbitbot::bot::qb_chat::QbChat;
use qbitbot::bot::qb_client::QbClient;

mod common;

#[tokio::test]
async fn run_simple_tests() {
    let test_case = TestCase::new().await;
    let tg_arc = test_case.get_tg_arc();
    test_menu_walk(tg_arc.clone()).await;
    test_download(tg_arc.clone()).await;
    test_torrent_page(tg_arc).await
}

pub async fn check_goto(chat: &mut QbChat, mock_rbot: Arc<RutebotMock>, text: &str) {
    chat.select_goto(mock_rbot, text).await.unwrap_or_else(|_| {
        panic!(
            r#"Failed to go to "{}" from {}"#,
            text,
            chat.get_menu_pos().get_command()
        )
    })
}

pub async fn create_qbchat_mock() -> QbChat {
    let conf = QbConfig::load_path("tests/.env_tests");
    let qbclient = QbClient::new(&conf).await;
    QbChat::new(0, qbclient)
}

async fn test_menu_walk(tg_mock: Arc<RutebotMock>) {
    let mut chat = create_qbchat_mock().await;
    assert_eq!(chat.get_menu_pos(), Main);
    check_goto(&mut chat,tg_mock.clone(), "/help").await;
    assert_eq!(chat.get_menu_pos(), Help);
    check_goto(&mut chat,tg_mock.clone(), "qwer").await;
    assert_eq!(chat.get_menu_pos(), Help);
    check_goto(&mut chat,tg_mock.clone(), "/list").await;
    assert_eq!(chat.get_menu_pos(), List);
    check_goto(&mut chat,tg_mock.clone(), "qwer").await;
    assert_eq!(chat.get_menu_pos(), List);
    check_goto(&mut chat, tg_mock.clone(), "/back").await;
    assert_eq!(chat.get_menu_pos(), Main);
}

async fn test_download(tg_mock: Arc<RutebotMock>) {
    let mut chat = create_qbchat_mock().await;
    check_goto(&mut chat, tg_mock.clone(), "/download").await;
    assert_eq!(chat.get_menu_pos(), Download);
    check_goto(&mut chat, tg_mock.clone(), MAGNET_LINK).await;
    assert_eq!(chat.get_menu_pos(), Download);
    check_goto(&mut chat, tg_mock.clone(), "qwerty").await;
    assert_eq!(chat.get_menu_pos(), Download);
}

async fn test_torrent_page(tg_mock: Arc<RutebotMock>) {
    let mut chat = create_qbchat_mock().await;
    check_goto(&mut chat, tg_mock.clone(), "/download").await;
    assert_eq!(chat.get_menu_pos(), Download);
    check_goto(&mut chat, tg_mock.clone(), MAGNET_LINK).await;
    assert_eq!(chat.get_menu_pos(), Download);
    check_goto(&mut chat, tg_mock.clone(), "/torrent0").await;
    assert_eq!(chat.get_menu_pos(), TorrentPage(0));
    check_goto(&mut chat, tg_mock.clone(), "/pause").await;
    assert_eq!(chat.get_menu_pos(), TorrentPage(0));
    check_goto(&mut chat, tg_mock.clone(), "/resume").await;
    assert_eq!(chat.get_menu_pos(), TorrentPage(0));
    check_goto(&mut chat, tg_mock.clone(), "qwer").await;
    assert_eq!(chat.get_menu_pos(), TorrentPage(0));
    check_goto(&mut chat, tg_mock.clone(), "/back").await;
    assert_eq!(chat.get_menu_pos(), List);
    check_goto(&mut chat, tg_mock.clone(), "/back").await;
    assert_eq!(chat.get_menu_pos(), Main);
}
