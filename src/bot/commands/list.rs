use std::{
    convert::TryInto,
    fmt::Display,
    time::{Duration, SystemTime},
};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use serde_json::Value;

use crate::bot::{commands::cmd_list::QbList, qb_client::QbClient};

use super::{cmd_list::MaindataResponse, QbCommandAction};

#[derive(Debug, Clone)]
pub struct QListAction {
    records: Vec<QbListRecord>,
    maindata_response: MaindataResponse,
    hash: String,
}

impl QListAction {
    pub async fn new(client: &QbClient) -> Result<Self> {
        let records = Self::update_records(client).await?;
        Ok(QListAction {
            records,
            maindata_response: MaindataResponse::default(),
            hash: String::from(""),
        })
    }

    pub async fn get(client: &QbClient) -> Result<Value> {
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

    pub async fn check_and_update(&mut self, client: &QbClient) -> Result<()> {
        let resp = self.check_has_changes(client).await?;
        self.records = Self::update_records(client).await?;
        Ok(())
    }

    pub async fn update_records(client: &QbClient) -> Result<Vec<QbListRecord>> {
        let resp = Self::get(client).await?;
        if let Some(arr) = resp.as_array() {
            let records: Vec<QbListRecord> = arr
                .iter()
                .enumerate()
                .filter_map(|(num, item)| QbListRecord::parse_record(num, item))
                .collect();
            Ok(records)
        } else {
            Err(anyhow!("Failed to get cached list"))
        }
    }

    async fn check_has_changes(&self, client: &QbClient) -> Result<Value> {
        let resp = client
            .qpost("/api/v2/sync/maindata", self.maindata_response.to_owned())
            .await?
            .json()
            .await?;
        dbg!(&resp);
        Ok(resp)
    }
}

#[derive(Debug, Clone)]
pub struct QbListRecord {
    num: usize,
    name: String,
    size: u64,
    progress: u64,
    eta: String,
}

impl QbListRecord {
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

    pub fn parse_record(num: usize, item: &Value) -> Option<Self> {
        let progress = item.get("progress").unwrap().as_f64()? * 100.0;
        let record = Self {
            num,
            progress: progress as u64,
            name: Self::get_name(item)?,
            size: item.get("size")?.as_u64()? / 1048576,
            eta: Self::get_eta(item)?,
        };
        Some(record)
    }
}

impl Display for QbListRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "/torrent{num}<code> | {name:20} | {size:6} Mb | {progress:3}% | {eta:19}</code>",
            num = self.num,
            name = self.name,
            size = self.size,
            progress = self.progress,
            eta = self.eta
        )
    }
}

impl QbCommandAction for QListAction {
    fn action_result_to_string(&self) -> String {
        if !self.records.is_empty() {
            self.records
                .iter()
                .cloned()
                .map(|record| record.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "Failed to get Qbittorrent response".to_string()
        }
    }
}
