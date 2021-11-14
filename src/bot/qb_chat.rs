use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use fure::backoff::fixed;
use fure::policies::{attempts, backoff};
use itertools::Itertools;
use rutebot::client::Rutebot;
use rutebot::requests::SendMessage;
use tokio::sync::mpsc::Sender;

use crate::bot::commands::aux::get_name_by_id;
use crate::bot::commands::pause_resume::QPauseResumeAction;
use crate::bot::commands::simple::QHelp;
use crate::bot::commands::QbCommandAction;
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
    rbot: Rutebot,
    qbclient: Arc<RwLock<QbClient>>,
    commands_map: HashMap<String, MenuValue>,
    notifier_tx: Option<Sender<CheckType>>,
}

impl QbChat {
    pub fn new(chat_id: i64, rbot: Rutebot, qbclient: Arc<RwLock<QbClient>>) -> Self {
        let mut chat = Self {
            chat_id,
            rbot,
            qbclient,
            menu_pos: MenuTree::from(Main),
            commands_map: MenuValue::generate_cmds(),
            notifier_tx: None,
        };
        chat.notifier_tx = Some(Self::get_tx(chat.clone()));
        chat
    }

    async fn do_cmd(&self) -> String {
        match self.menu_pos.value {
            Main => "Main menu".to_string(),
            Help => QHelp {}.action_result_to_string(),
            List => self
                .qbclient
                .write()
                .unwrap()
                .get_cached_list()
                .await
                .unwrap()
                .action_result_to_string(),
            Download => "Send torrent link or attach torrent file".to_string(),
            TorrentPage(id) => {
                let name = get_name_by_id(&self.qbclient.read().unwrap(), id)
                    .await
                    .unwrap_or_else(|| "Failed to get name".to_string());
                format!("Torrent management: {}", name)
            }
            _ => "You will never see this message".to_string(),
        }
    }

    pub async fn select_goto(&mut self, text: &str) {
        match text {
            "/back" => self.back().await,
            command if self.commands_map.contains_key(command) => {
                self.goto(self.commands_map.get(command).unwrap().to_owned())
                    .await
            }
            _ if text.starts_with("/torrent") => {
                if let Ok(id) = text.strip_prefix("/torrent").unwrap().parse::<usize>() {
                    self.goto(TorrentPage(id)).await
                } else {
                    self.goto(self.menu_pos.value.clone()).await
                }
            }
            cmd @ ("/pause" | "/resume") if matches!(self.menu_pos.value, TorrentPage(_)) => {
                let res = if let TorrentPage(id) = self.menu_pos.value {
                    QPauseResumeAction::new(cmd.strip_prefix('/').unwrap())
                        .act(&self.qbclient.read().unwrap(), id)
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
                self.send_message(message).await
            }
            _ => match self.menu_pos.value {
                Download => {
                    let download_obj = QDownloadAction::new()
                        .send_link(&self.qbclient.read().unwrap(), text)
                        .await
                        .unwrap();
                    download_obj
                        .create_notifier(
                            &self.qbclient.read().unwrap(),
                            self.notifier_tx.clone().unwrap(),
                        )
                        .await;
                    let res = download_obj.action_result_to_string();
                    let message = MessageWrapper {
                        text: res,
                        parse_mode: Some(rutebot::requests::ParseMode::Html),
                    };
                    self.send_message(message).await
                }
                _ => self.goto(self.menu_pos.value.clone()).await,
            },
        };
    }

    async fn goto(&mut self, menu_value: MenuValue) {
        self.menu_pos = MenuTree::from(menu_value);
        let content = self.do_cmd().await;
        let text = self.menu_pos.show(content).await;
        let message = MessageWrapper {
            text,
            parse_mode: Some(rutebot::requests::ParseMode::Html),
        };
        self.send_message(message).await;
    }

    async fn back(&mut self) {
        if self.menu_pos.parent.is_some() {
            self.goto(self.menu_pos.parent.clone().unwrap()).await;
        } else {
            self.goto(self.menu_pos.value.clone()).await
        }
    }

    pub async fn send_message(&self, message: MessageWrapper) {
        debug!("Sending message: {:#?}", message);
        let send_msg = || async {
            let reply = SendMessage {
                parse_mode: message.parse_mode,
                ..SendMessage::new(self.chat_id, &message.text)
            };
            self.rbot.clone().prepare_api_request(reply).send().await
        };
        let policy = attempts(backoff(fixed(Duration::from_secs(3))), 3);
        if fure::retry(send_msg, policy).await.is_err() {
            error!("Failed to send reply 4 times")
        };
    }
}

impl Notifier for QbChat {}
