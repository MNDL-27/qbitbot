use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use fure::backoff::fixed;
use fure::policies::{attempts, backoff};
use itertools::Itertools;
use rutebot::client::Rutebot;
use rutebot::requests::SendMessage;
use tokio::sync::mpsc::Sender;

use crate::bot::commands::pause_resume::QPauseResumeAction;
use crate::bot::commands::QbCommandAction;
use crate::bot::commands::simple::QHelp;
use crate::bot::notifier::CheckType;
use crate::bot::qb_chat::MenuValue::*;
use crate::bot::qb_client::QbClient;
use crate::bot::qbot::MessageWrapper;

use super::commands::download::QDownloadAction;
use super::notifier::Notifier;

#[derive(Clone, Debug, PartialOrd, Ord, Eq, PartialEq)]
pub enum MenuValue {
    Main,
    Help,
    List,
    Download,
    TorrentPage(usize),
    Pause,
    Resume,
}

pub static COMMANDS: &[MenuValue] = &[Main, Help, List, Download];

impl MenuValue {
    pub fn get_command(&self) -> &str {
        match self {
            Main => "/main",
            Help => "/help",
            List => "/list",
            Download => "/download",
            TorrentPage(_) => "/torrent",
            Pause => "/pause",
            Resume => "/resume,",
        }
    }

    pub fn get_help(&self) -> &str {
        match self {
            Main => "Go to main menu",
            Help => "Show help for all commands",
            List => "List torrents",
            Download => "Start downloading by link or attached file",
            TorrentPage(_) => "Show torrent page",
            _ => "",
        }
    }

    pub fn generate_cmds() -> HashMap<String, MenuValue> {
        COMMANDS
            .iter()
            .cloned()
            .map(|v| (v.get_command().to_string(), v))
            .collect()
    }
}

#[derive(Clone, Debug)]
struct MenuTree {
    value: MenuValue,
    parent: Option<MenuValue>,
    children: Vec<MenuValue>,
}

impl From<MenuValue> for MenuTree {
    fn from(value: MenuValue) -> Self {
        match value {
            Main => MenuTree {
                value,
                parent: None,
                children: vec![Help, List, Download],
            },
            Help => MenuTree {
                value,
                parent: Some(Main),
                children: vec![],
            },
            List => MenuTree {
                value,
                ..MenuTree::from(Help)
            },
            Download => MenuTree {
                value,
                ..MenuTree::from(Help)
            },
            TorrentPage(_) => MenuTree {
                value,
                parent: Some(List),
                children: vec![Pause, Resume],
            },
            Pause => MenuTree {
                value,
                ..MenuTree::from(Pause)
            },
            Resume => MenuTree {
                value,
                ..MenuTree::from(Resume)
            },
        }
    }
}

impl MenuTree {
    pub async fn show(&self, content: String) -> String {
        format!(
            "{}\n\nButtons:\n{}\n/back",
            content,
            self.print_children().as_str()
        )
    }

    fn print_children(&self) -> String {
        self.children
            .iter()
            .sorted()
            .map(|v| v.get_command())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Clone)]
pub struct QbChat {
    chat_id: i64,
    menu_pos: MenuTree,
    rbot: Option<Rutebot>,
    qbclient: QbClient,
    commands_map: HashMap<String, MenuValue>,
    notifier_tx: Option<Sender<CheckType>>,
}

impl QbChat {
    pub fn new(chat_id: i64, rbot: Rutebot, qbclient: QbClient) -> Self {
        let mut chat = Self {
            chat_id,
            qbclient,
            rbot: Some(rbot),
            menu_pos: MenuTree::from(Main),
            commands_map: MenuValue::generate_cmds(),
            notifier_tx: None,
        };
        chat.notifier_tx = Some(Self::get_tx(chat.clone()));
        chat
    }

    async fn do_cmd(&mut self) -> Result<String> {
        let res = match self.menu_pos.value {
            Main => "Main menu".to_string(),
            Help => QHelp {}.action_result_to_string(),
            List => self
                .qbclient
                .get_cached_list()
                .await?
                .action_result_to_string(),
            Download => "Send torrent link or attach torrent file".to_string(),
            TorrentPage(id) => {
                if let Some(record) = self.qbclient.get_cached_list().await?.get_record_by_num(id) {
                    record.to_string()
                } else {
                    "There is no torrent with this id".to_string()
                }
            }
            _ => "You will never see this message".to_string(),
        };
        Ok(res)
    }

    pub async fn select_goto(&mut self, text: &str) -> Result<()> {
        match text {
            "/back" => self.back().await?,
            command if self.commands_map.contains_key(command) => {
                self.goto(self.commands_map.get(command).unwrap().to_owned())
                    .await?
            }
            _ if text.starts_with("/torrent") => {
                if let Ok(id) = text.strip_prefix("/torrent").unwrap().parse::<usize>() {
                    self.goto(TorrentPage(id)).await?
                } else {
                    self.goto(self.menu_pos.value.clone()).await?
                }
            }
            cmd @ ("/pause" | "/resume") if matches!(self.menu_pos.value, TorrentPage(_)) => {
                let res = if let TorrentPage(id) = self.menu_pos.value {
                    QPauseResumeAction::new(cmd.strip_prefix('/').unwrap())
                        .act(&mut self.qbclient, id)
                        .await
                        .action_result_to_string()
                } else {
                    // dummy code to complete if let else
                    String::from("")
                };
                let message = MessageWrapper {
                    text: res,
                    parse_mode: None,
                };
                self.chat_send_message(message).await
            }
            _ => match self.menu_pos.value {
                Download => {
                    let download_obj = QDownloadAction::new()
                        .send_link(&self.qbclient, text)
                        .await?;
                    if let Some(tx) = self.notifier_tx.clone() {
                        download_obj.create_notifier(&mut self.qbclient, tx).await;
                    } else {
                        error!("Notifier for chat {} is not initialized", self.chat_id)
                    }
                    let res = download_obj.action_result_to_string();
                    let message = MessageWrapper {
                        text: res,
                        parse_mode: Some(rutebot::requests::ParseMode::Html),
                    };
                    self.chat_send_message(message).await
                }
                _ => self.goto(self.menu_pos.value.clone()).await?,
            },
        };
        Ok(())
    }

    async fn goto(&mut self, menu_value: MenuValue) -> Result<()> {
        self.menu_pos = MenuTree::from(menu_value);
        let content = self.do_cmd().await?;
        let text = self.menu_pos.show(content).await;
        let message = MessageWrapper {
            text,
            parse_mode: Some(rutebot::requests::ParseMode::Html),
        };
        self.chat_send_message(message).await;
        Ok(())
    }

    async fn back(&mut self) -> Result<()> {
        if self.menu_pos.parent.is_some() {
            self.goto(self.menu_pos.parent.clone().unwrap()).await?;
        } else {
            self.goto(self.menu_pos.value.clone()).await?
        }
        Ok(())
    }

    pub async fn relogin(&mut self) -> Result<()> {
        self.qbclient.login().await
    }

    pub async fn chat_send_message(&self, message: MessageWrapper) {
        if let Some(rbot) = &self.rbot {
            send_message(rbot, self.chat_id, message).await
        }
    }
}

pub async fn send_message(rbot: &Rutebot, chat_id: i64, message: MessageWrapper) {
    debug!("Sending message to chat({}): {:#?}", chat_id, message);
    let send_msg = || async {
        let reply = SendMessage {
            parse_mode: message.parse_mode,
            ..SendMessage::new(chat_id, &message.text)
        };
        rbot.clone().prepare_api_request(reply).send().await
    };
    let policy = attempts(backoff(fixed(Duration::from_secs(3))), 3);
    if fure::retry(send_msg, policy).await.is_err() {
        error!("Failed to send reply 4 times")
    };
}

impl Notifier for QbChat {}

#[cfg(test)]
mod tests {
    use crate::bot::config::QbConfig;

    use super::*;

    impl QbChat {
        pub fn new_mock(qbclient: QbClient) -> Self {
            Self {
                qbclient,
                chat_id: -1,
                rbot: None,
                menu_pos: MenuTree::from(Main),
                commands_map: MenuValue::generate_cmds(),
                notifier_tx: None,
            }
        }

        pub async fn check_goto(&mut self, text: &str) {
            self.select_goto(text).await.unwrap_or_else(|_| {
                panic!(
                    r#"Failed to go to "{}" from {}"#,
                    text,
                    self.menu_pos.value.get_command()
                )
            })
        }
    }

    async fn create_qbchat() -> QbChat {
        let conf = QbConfig::load_path("tests/.env_tests");
        let qbclient = QbClient::new(&conf).await;
        QbChat::new_mock(qbclient)
    }

    #[tokio::test]
    async fn test_menu_walk() {
        let mut chat = create_qbchat().await;
        assert_eq!(chat.menu_pos.value, Main);
        chat.check_goto("/help").await;
        assert_eq!(chat.menu_pos.value, Help);
        chat.check_goto("qwer").await;
        assert_eq!(chat.menu_pos.value, Help);
        chat.check_goto("/list").await;
        assert_eq!(chat.menu_pos.value, List);
        chat.check_goto("qwer").await;
        assert_eq!(chat.menu_pos.value, List);
        chat.check_goto("/back").await;
        assert_eq!(chat.menu_pos.value, Main);
    }

    #[tokio::test]
    async fn test_download() {
        let mut chat = create_qbchat().await;
        chat.check_goto("/download").await;
        assert_eq!(chat.menu_pos.value, Download);
        let magnet_link = "magnet:?xt=urn:btih:60A2A94625373B5ACAE66D4C693AE5F3417690C1&tr=http%3A%2F%2Fbt3.t-ru.org%2Fann%3Fmagnet&dn=Peter%20Bruce%2C%20Andrew%20Bruce%2C%20Peter%20Gedeck%20%2F%20Питер%20Брюс%2C%20Эндрю%20Брюс%2C%20Питер%20Гедек%20-%20Practical%20Statistics%20for%20Data%20Scientists%20%2F%20Практическая%20статистика%20для";
        chat.check_goto(magnet_link).await;
        assert_eq!(chat.menu_pos.value, Download);
        chat.check_goto("qwerty").await;
        assert_eq!(chat.menu_pos.value, Download);
    }

    #[tokio::test]
    async fn test_torrent_page() {
        let mut chat = create_qbchat().await;
        chat.check_goto("/download").await;
        assert_eq!(chat.menu_pos.value, Download);
        let magnet_link = "magnet:?xt=urn:btih:60A2A94625373B5ACAE66D4C693AE5F3417690C1&tr=http%3A%2F%2Fbt3.t-ru.org%2Fann%3Fmagnet&dn=Peter%20Bruce%2C%20Andrew%20Bruce%2C%20Peter%20Gedeck%20%2F%20Питер%20Брюс%2C%20Эндрю%20Брюс%2C%20Питер%20Гедек%20-%20Practical%20Statistics%20for%20Data%20Scientists%20%2F%20Практическая%20статистика%20для";
        chat.check_goto(magnet_link).await;
        assert_eq!(chat.menu_pos.value, Download);
        chat.check_goto("/torrent0").await;
        assert_eq!(chat.menu_pos.value, TorrentPage(0));
        chat.check_goto("/pause").await;
        assert_eq!(chat.menu_pos.value, TorrentPage(0));
        chat.check_goto("/resume").await;
        assert_eq!(chat.menu_pos.value, TorrentPage(0));
        chat.check_goto("qwer").await;
        assert_eq!(chat.menu_pos.value, TorrentPage(0));
        chat.check_goto("/back").await;
        assert_eq!(chat.menu_pos.value, List);
        chat.check_goto("/back").await;
        assert_eq!(chat.menu_pos.value, Main);
    }
}
