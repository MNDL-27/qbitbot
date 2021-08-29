use serde::{Deserialize, Serialize};

use super::qb_client::QbClient;
use anyhow::Result;

pub trait QbCommand {
    /// Command name as it should be written in Telegram chat
    fn name() -> String;
    /// Command description to show in /help message
    fn description() -> String {
        "No description".to_string()
    }
}

// TODO: macro is needed
pub fn gen_help() -> String {
    "Help".to_string()
}

#[derive(Serialize, Deserialize)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct QbList {
    filter: String,
}

impl QbCommand for QbList {
    fn name() -> String {
        "/list".to_string()
    }

    fn description() -> String {
        "List all torrents".to_string()
    }
}

impl QbList {
    pub async fn get(client: &QbClient, filter: &str) -> Result<String> {
        let resp = client
            .qsend(
                "query/torrents",
                QbList {
                    filter: filter.to_string(),
                },
            )
            .await?
            .text()
            .await?;
        Ok(resp)
    }
}
