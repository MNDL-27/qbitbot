use std::time::Duration;

use crate::bot::{commands::list::QListAction, qb_client::QbClient};

use super::{cmd_list::QPause, QbCommandAction};
use anyhow::{anyhow, Result};
use tokio::time::sleep;

pub struct QPauseAction {
    pub status: bool,
}

impl QPauseAction {
    async fn check_paused(client: &QbClient, hash: &str) -> Option<()> {
        sleep(Duration::from_millis(1000)).await;
        let items = QListAction::new().get(client).await.ok()?;
        let item = items
            .as_array()?
            .iter()
            .find(|item| item.get("hash").unwrap() == hash)?;
        let state = item.as_object()?.get("state")?.to_owned();
        if state.to_string().contains("paused") {
            Some(())
        } else {
            None
        }
    }

    async fn id_to_hash(client: &QbClient, id: usize) -> Option<String> {
        let array = QListAction::new()
            .get(client)
            .await
            .ok()?
            .as_array()?
            .to_owned();
        let (_, item) = array.iter().enumerate().nth(id)?;
        let hash = item.as_object()?.get("hash")?.as_str()?.to_string();
        Some(hash)
    }

    pub async fn act(mut self, client: &QbClient, id: usize) -> Result<Self> {
        let hash = Self::id_to_hash(client, id)
            .await
            .ok_or_else(|| anyhow!("ID to hash conversion failed"))?;
        client
            .qpost(
                "/command/pause",
                QPause {
                    hash: hash.to_string(),
                },
            )
            .await?;
        self.status = Self::check_paused(client, &hash).await.is_some();
        Ok(self)
    }
}

impl QbCommandAction for QPauseAction {
    fn name(&self) -> String {
        "/pause".to_string()
    }

    fn action_result_to_string(&self) -> String {
        if self.status { "OK" } else { "FAIL" }.to_string()
    }
}
