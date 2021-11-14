use std::time::Duration;

use anyhow::{anyhow, Result};
use fure::backoff::fixed;
use fure::policies::{attempts, backoff};
use serde_json::Value;

use crate::bot::commands::aux::id_to_hash;
use crate::bot::commands::list::QListAction;
use crate::bot::qb_client::QbClient;

use super::{cmd_list::QPause, QbCommandAction};

pub struct QPauseResumeAction {
    status: Result<()>,
    action: String,
}

impl QPauseResumeAction {
    /// Only two action types allowed: "pause" and "resume"
    ///
    /// Example:
    /// ```
    /// QPauseResumeAction::new("pause")
    /// ```
    pub fn new(action: &str) -> Self {
        Self {
            status: Ok(()),
            action: String::from(action),
        }
    }

    fn check_paused(&self, state: &str) -> Result<()> {
        if state.contains("paused") {
            Ok(())
        } else {
            Err(anyhow!(format!("Failed to {}", self.action)))
        }
    }

    fn check_resumed(&self, state: &str) -> Result<()> {
        if !state.contains("paused") {
            Ok(())
        } else {
            Err(anyhow!(format!("Failed to {}", self.action)))
        }
    }

    fn check(&self, state: &str) -> Result<()> {
        if self.action == "pause" {
            self.check_paused(state)
        } else {
            self.check_resumed(state)
        }
    }

    async fn check_state(&self, client: &QbClient, hash: &str) -> Result<()> {
        let parse_and_check_state = |items: Value| {
            let item = items
                .as_array()?
                .iter()
                .find(|item| item.get("hash").unwrap() == hash)?;
            let state = item.as_object()?.get("state")?.to_owned().to_string();
            Some(state)
        };
        let get_and_check_state = || async {
            let items = QListAction::get(client).await?;
            let state =
                parse_and_check_state(items).ok_or_else(|| anyhow!("Failed to parse state"))?;
            self.check(&state)
        };
        let policy = attempts(backoff(fixed(Duration::from_millis(500))), 3);
        fure::retry(get_and_check_state, policy).await
    }

    pub async fn act(mut self, client: &QbClient, id: usize) -> Self {
        let hash_opt = id_to_hash(client, id).await;
        if hash_opt.is_none() {
            self.status = Err(anyhow!("ID to hash conversion failed"));
            return self;
        }
        let hash = hash_opt.unwrap();
        let qpost_res = client
            .qpost(
                format!("/torrents/{}", self.action).as_str(),
                QPause {
                    hashes: hash.to_string(),
                },
            )
            .await;
        if qpost_res.is_err() {
            self.status = Err(anyhow!("Failed to send request to Qbittorrent"))
        }
        self.status = self.check_state(client, &hash).await;
        self
    }
}

impl QbCommandAction for QPauseResumeAction {
    fn action_result_to_string(&self) -> String {
        if let Err(error) = &self.status {
            error.to_string()
        } else {
            String::from("OK")
        }
    }
}
