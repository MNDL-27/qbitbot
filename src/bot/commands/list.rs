use std::{
    convert::TryInto,
    time::{Duration, SystemTime},
};

use anyhow::Result;
use chrono::{DateTime, Local};
use serde_json::Value;

use crate::bot::{commands::cmd_list::QbList, qb_client::QbClient};

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
        let eta = item.get("eta")?.as_i64()?;
        let completion_on = item.get("completion_on")?.as_i64()?;
        let humanized_eta = match (eta, completion_on) {
            (8640000, -10800) => "waiting".to_string(),
            (8640000, _) => "done".to_string(),
            _ => {
                let secs = SystemTime::now() + Duration::from_secs(eta.try_into().ok()?);
                DateTime::<Local>::from(secs)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            }
        };
        Some(humanized_eta)
    }

    pub async fn get(&self, client: &QbClient) -> Result<Value> {
        let resp = client
            .qpost(
                "/torrents/info",
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
                        "/torrent{num}<code> | {name:20} | {size:6} Mb | {progress:3}% | {eta:19}</code>\n",
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
    fn action_result_to_string(&self) -> String {
        self.content
            .clone()
            .map_or("Failed to get Qbittorrent response".to_string(), |x| x)
    }
}
