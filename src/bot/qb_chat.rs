use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use itertools::Itertools;

use crate::bot::commands::download::QDownloadAction;
use crate::bot::commands::pause_resume::QPauseResumeAction;
use crate::bot::commands::QbCommandAction;
use crate::bot::commands::simple::QHelp;
use crate::bot::messages::TelegramBackend;
use crate::bot::notifier::Notifier;
use crate::bot::qb_chat::MenuValue::*;
use crate::bot::qb_client::QbClient;
use crate::bot::qbot::MessageWrapper;

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
    qbclient: QbClient,
    commands_map: HashMap<String, MenuValue>,
}

impl QbChat {
    pub fn new(chat_id: i64, qbclient: QbClient) -> Self {
        Self {
            chat_id,
            qbclient,
            menu_pos: MenuTree::from(Main),
            commands_map: MenuValue::generate_cmds(),
        }
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

    pub async fn select_goto(&mut self, rbot: Arc<dyn TelegramBackend>, text: &str) -> Result<()> {
        match text {
            "/back" => self.back(rbot).await?,
            command if self.commands_map.contains_key(command) => {
                self.goto(rbot, self.commands_map.get(command).unwrap().to_owned())
                    .await?
            }
            _ if text.starts_with("/torrent") => {
                if let Ok(id) = text.strip_prefix("/torrent").unwrap().parse::<usize>() {
                    self.goto(rbot, TorrentPage(id)).await?
                } else {
                    self.goto(rbot, self.menu_pos.value.clone()).await?
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
                rbot.send_message(self.chat_id, message).await
            }
            _ => match self.menu_pos.value {
                Download => {
                    let download_obj = QDownloadAction::default()
                        .send_link(&self.qbclient, text)
                        .await?;
                    let tx = Self::create_notifier_tx(rbot.clone(), self.chat_id);
                    download_obj.create_notifier(&mut self.qbclient, tx).await;
                    let res = download_obj.action_result_to_string();
                    let message = MessageWrapper {
                        text: res,
                        parse_mode: Some(rutebot::requests::ParseMode::Html),
                    };
                    rbot.send_message(self.chat_id, message).await
                }
                _ => self.goto(rbot, self.menu_pos.value.clone()).await?,
            },
        };
        Ok(())
    }

    async fn goto(&mut self, rbot: Arc<dyn TelegramBackend>, menu_value: MenuValue) -> Result<()> {
        self.menu_pos = MenuTree::from(menu_value);
        let content = self.do_cmd().await?;
        let text = self.menu_pos.show(content).await;
        let message = MessageWrapper {
            text,
            parse_mode: Some(rutebot::requests::ParseMode::Html),
        };
        rbot.send_message(self.chat_id, message).await;
        Ok(())
    }

    async fn back(&mut self, rbot: Arc<dyn TelegramBackend>) -> Result<()> {
        if self.menu_pos.parent.is_some() {
            self.goto(rbot, self.menu_pos.parent.clone().unwrap())
                .await?;
        } else {
            self.goto(rbot, self.menu_pos.value.clone()).await?
        }
        Ok(())
    }

    pub async fn relogin(&mut self) -> Result<()> {
        self.qbclient.login().await
    }

    pub fn get_menu_pos(&self) -> MenuValue {
        self.menu_pos.value.clone()
    }
}

impl Notifier for QbChat {}
