use std::{
    convert::TryInto,
    fmt::Display,
    time::{Duration, SystemTime},
};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use serde_json::{json, Value};

use crate::bot::commands::cmd_list::QGetProperties;
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

    pub async fn get_properties(client: &QbClient, hash: String) -> Result<Value> {
        let value = client
            .qpost("/torrents/properties", QGetProperties { hash })
            .await?
            .json()
            .await?;
        Ok(value)
    }

    pub async fn check_and_update(&mut self, client: &QbClient) -> Result<()> {
        match self.check_has_changes(client).await {
            Err(_) | Ok(true) => {
                debug!("Need to update cached list");
                self.records = Self::update_records(client).await?
            }
            Ok(false) => (),
        };
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

    async fn check_has_changes(&mut self, client: &QbClient) -> Result<bool> {
        let resp: Value = client
            .qpost("/sync/maindata", self.maindata_response.to_owned())
            .await?
            .json()
            .await?;

        self.maindata_response.rid = (|| -> Option<i64> {
            let res = resp.as_object()?.get("rid")?.as_i64()?;
            Some(res)
        })()
        .ok_or_else(|| anyhow!("Failed to parse Qbittorrent response"))?;

        let updated = (|| -> Option<bool> {
            let updated = resp
                .as_object()?
                .get("full_update")
                .unwrap_or(&json!("false"))
                .as_bool()
                .unwrap_or(false);
            Some(updated)
        })()
        .ok_or_else(|| anyhow!("Failed to parse Qbittorrent response"))?;

        Ok(updated)
    }

    pub fn get_record_by_num(&self, num: usize) -> Option<QbListRecord> {
        self.records.iter().find(|&item| item.num == num).cloned()
    }

    pub fn get_records(&self) -> &Vec<QbListRecord> {
        &self.records
    }
}

#[derive(Debug, Clone)]
pub struct QbListRecord {
    num: usize,
    name: String,
    size: u64,
    progress: u64,
    eta: String,
    hash: String,
}

impl QbListRecord {
    fn parse_name(item: &Value) -> Option<String> {
        let name = item
            .get("name")?
            .to_string()
            .chars()
            .filter(|&x| x != '"')
            .take(20)
            .collect();
        Some(name)
    }

    fn parse_eta(item: &Value) -> Option<String> {
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
            name: Self::parse_name(item)?,
            size: item.get("size")?.as_u64()? / 1048576,
            eta: Self::parse_eta(item)?,
            hash: item.get("hash")?.as_str()?.to_string(),
        };
        Some(record)
    }

    pub fn get_hash(&self) -> String {
        self.hash.to_owned()
    }

    pub fn get_name(&self) -> String {
        self.name.to_owned()
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
