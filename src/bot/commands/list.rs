use std::time::{Duration, SystemTime};

use chrono::{DateTime, Local};
use rutebot::requests::ParseMode;

use anyhow::Result;
use serde_json::Value;

use crate::bot::{commands::cmd_list::QbList, qb_client::QbClient, qbot::RbotParseMode};

use super::QbCommandAction;

pub struct QListAction {
    content: Option<String>,
}

impl QListAction {
    pub fn new() -> Self {
        QListAction { content: None }
    }

    fn get_name(item: &Value) -> Option<String> {
        let name = item
            .get("name")?
            .to_string()
            .chars()
            .filter(|&x| x != '"')
            .take(20)
            .collect();
        Some(name)
    }

    fn get_eta(item: &Value) -> Option<String> {
        let eta = item.get("eta")?.as_u64()?;
        let humanized_eta = if eta == 8640000 {
            "done".to_string()
        } else {
            let secs = SystemTime::now() + Duration::from_secs(eta);
            DateTime::<Local>::from(secs)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        };
        Some(humanized_eta)
    }

    pub async fn get(&self, client: &QbClient) -> Result<Value> {
        let resp = client
            .qget(
                "/query/torrents",
                QbList {
                    sort: "hash".to_string(),
                },
            )
            .await?
            .json()
            .await?;
        Ok(resp)
    }

    pub async fn get_formatted(mut self, client: &QbClient) -> Result<Self> {
        let resp = self.get(client).await?;
        self.content = move || -> Option<String> {
            let converted = resp
                .as_array()?
                .iter()
                .enumerate()
                .filter_map(|(num, item)| {
                    let progress = item.get("progress").unwrap().as_f64()? * 100.0;
                    Some(format!(
                        "{num:2} | {name:20} | {size:6} Mb | {progress:3}% | {eta:19}\n",
                        num = num,
                        name = Self::get_name(item)?,
                        size = item.get("size")?.as_u64()? / 1048576,
                        progress = progress as u64,
                        eta = Self::get_eta(item)?
                    ))
                })
                .collect();
            Some(converted)
        }();
        Ok(self)
    }
}

impl QbCommandAction for QListAction {
    fn name(&self) -> String {
        "/list".to_string()
    }

    fn description(&self) -> String {
        "List all torrents".to_string()
    }

    fn parse_mode(&self) -> RbotParseMode {
        Some(ParseMode::Html)
    }

    fn action_result_to_string(&self) -> String {
        self.content
            .clone()
            .map_or("Failed to get Qbittorrent response".to_string(), |x| x)
    }
}
