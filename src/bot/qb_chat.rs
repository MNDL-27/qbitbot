use std::collections::HashMap;
use std::time::Duration;

use fure::backoff::fixed;
use fure::policies::{attempts, backoff};
use rutebot::client::Rutebot;
use rutebot::requests::SendMessage;

use crate::bot::qb_chat::MenuValue::*;
use crate::bot::qbot::MessageWrapper;

#[derive(Clone, Debug)]
enum MenuValue {
    Main,
    Torrent,
    Help,
    List,
    Download,
}

static COMMANDS: &[MenuValue] = &[Main, Torrent, Help, List, Download];

impl MenuValue {
    pub fn get_command(&self) -> &str {
        match self {
            Main => "/main",
            Torrent => "/torrent",
            Help => "/help",
            List => "/list",
            Download => "/download"
        }
    }

    pub fn get_menu_content(&self) -> String {
        match self {
            Main => "Main menu".to_string(),
            Torrent => "Torrent menu".to_string(),
            Help => "Help is here".to_string(),
            List => "List".to_string(),
            Download => "Download".to_string()
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
                children: vec![Torrent, Help, List, Download],
            },
            Help => MenuTree {
                value,
                parent: Some(Main),
                children: vec![],
            },
            Torrent => MenuTree {
                value,
                ..MenuTree::from(Help)
            },
            List => MenuTree {
                value,
                ..MenuTree::from(Help)
            },
            Download => MenuTree {
                value,
                ..MenuTree::from(Help)
            }
        }
    }
}

impl MenuTree {
    pub fn show(&self) -> String {
        let content = self.value.get_menu_content();
        format!("{}\nButtons:\n{}\n/back", content, self.print_children().as_str())
    }

    fn print_children(&self) -> String {
        self.children.iter().map(|v| v.get_command()).collect::<Vec<_>>().join("\n")
    }
}

#[derive(Clone)]
pub struct QbChat {
    chat_id: i64,
    menu_pos: MenuTree,
    rbot: Rutebot,
    commands_map: HashMap<String, MenuValue>,
}

impl QbChat {
    pub fn new(chat_id: i64, rbot: Rutebot) -> Self {
        Self {
            chat_id,
            rbot,
            menu_pos: MenuTree::from(Main),
            commands_map: MenuValue::generate_cmds(),
        }
    }

    pub async fn select_goto(&mut self, text: &str) {
        match text {
            "/back" => self.back().await,
            command if self.commands_map.contains_key(command) =>
                self.goto(self.commands_map.get(command).unwrap().to_owned()).await,
            _ => self.goto(self.menu_pos.value.clone()).await
        };
    }

    async fn goto(&mut self, menu_value: MenuValue) {
        self.menu_pos = MenuTree::from(menu_value);
        let text = self.menu_pos.show();
        let message = MessageWrapper {
            text,
            parse_mode: Some(rutebot::requests::ParseMode::Markdown),
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