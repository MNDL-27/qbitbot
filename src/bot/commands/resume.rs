use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::time::sleep;

use crate::bot::{
    commands::{cmd_list::QResume, list::QListAction},
    qb_client::QbClient,
};

use super::QbCommandAction;

pub struct QResumeAction {
    status: Result<()>,
}

impl QResumeAction {
    pub fn new() -> Self {
        Self { status: Ok(()) }
    }

    async fn check_resumed(client: &QbClient, hash: &str) -> Option<()> {
        sleep(Duration::from_millis(1000)).await;
        let items = QListAction::new().get(client).await.ok()?;
        let item = items
            .as_array()?
            .iter()
            .find(|item| item.get("hash").unwrap() == hash)?;
        let state = item.as_object()?.get("state")?.to_owned();
        debug!("Hash {} state: {}", hash, state);
        if !state.as_str()?.contains("paused") {
            Some(())
        } else {
            None
        }
    }

    pub async fn act(mut self, arc_client: Arc<QbClient>, id: usize) -> Self {
        let client = arc_client.deref();
        let hash_opt = QListAction::id_to_hash(client, id).await;
        if hash_opt.is_none() {
            self.status = Err(anyhow!("ID to hash conversion failed"));
            return self;
        }
        let hash = hash_opt.unwrap();
        let qpost_res = client
            .qpost(
                "/torrents/resume",
                QResume {
                    hashes: hash.to_string(),
                },
            )
            .await;
        if qpost_res.is_err() {
            self.status = Err(anyhow!("Failed to send request to Qbittorrent"))
        }
        self.status = Self::check_resumed(client, &hash)
            .await
            .ok_or_else(|| anyhow!("Failed to resume"));
        self
    }
}

impl QbCommandAction for QResumeAction {
    fn action_result_to_string(&self) -> String {
        if let Err(error) = &self.status {
            error.to_string()
        } else {
            String::from("OK")
        }
    }
}
