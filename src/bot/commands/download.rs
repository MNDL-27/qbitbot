use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use fure::backoff::fixed;
use fure::policies::{attempts, backoff};
use reqwest::Response;
use tokio::sync::mpsc::Sender;
use tokio::time::{Duration, sleep};

use crate::bot::{qb_client::QbClient, TAG_NAME};
use crate::bot::notifier::CheckType;
use crate::bot::notifier::CheckType::Completed;

use super::{cmd_list::QDownload, list::QListAction, QbCommandAction};

pub struct QDownloadAction {
    status: bool,
    torrent_hash: String,
    torrent_name: String,
}

impl QDownloadAction {
    pub fn new() -> Self {
        QDownloadAction {
            status: false,
            torrent_hash: "".to_string(),
            torrent_name: "".to_string(),
        }
    }

    async fn check_added(
        &mut self,
        client: &QbClient,
        list_before: Option<HashSet<String>>,
    ) -> Result<()> {
        if let Some(hashes) = list_before {
            let closure = || async {
                let list_after = Self::get_hashes(client).await.unwrap_or_default();
                let diff: HashSet<String> = list_after.difference(&hashes).cloned().collect();
                // TODO: fix this barely good check
                if diff.len() == 1 {
                    Ok(diff.iter().cloned().next().unwrap())
                } else {
                    Err(())
                }
            };
            let policy = attempts(backoff(fixed(Duration::from_millis(500))), 3);
            let res = fure::retry(closure, policy).await;
            if let Ok(hash) = res {
                self.torrent_hash = hash;
                Ok(())
            } else {
                Err(anyhow!("Torrent has not been added"))
            }
        } else {
            Err(anyhow!("Failed to check added torrent"))
        }
    }

    async fn inner_send(client: &QbClient, link: &str) -> Result<Response> {
        let resp = client
            .qpost(
                "/torrents/add",
                QDownload {
                    urls: link.to_string(),
                    tags: TAG_NAME.to_string(),
                },
            )
            .await?;
        Ok(resp)
    }

    async fn get_hashes(client: &QbClient) -> Option<HashSet<String>> {
        let hashes = QListAction::new()
            .get(client)
            .await
            .ok()?
            .as_array()?
            .iter()
            .filter_map(|item| serde_json::from_value(item.get("hash").cloned()?).ok())
            .collect();
        Some(hashes)
    }

    pub async fn get_name(client: &QbClient, hash: &str) -> Option<String> {
        let list_info = QListAction::new().get(client).await.ok()?;
        let name = list_info
            .as_array()?
            .iter()
            .find(|item| item.get("hash").unwrap() == hash)?
            .as_object()?
            .get("name")?
            .to_string();
        Some(name)
    }

    async fn check_is_completed(client: &QbClient, hash: &str, name: &str) -> Result<String> {
        let res = if let Ok(props) = client.get_properties(hash.to_string()).await {
            let completion_date_opt = move || -> Option<i64> {
                let obj = props.as_object()?;
                obj.get("completion_date")?.as_i64()
            }();
            if let Some(completion_date) = completion_date_opt {
                if completion_date != -1 {
                    Ok(format!("{} is done", name))
                } else {
                    Err(anyhow!("Torrent in progress"))
                }
            } else {
                Err(anyhow!("Failed to get torrent status"))
            }
        } else {
            Err(anyhow!("Failed to get torrent status"))
        };
        debug!("Checked {} for completion", name);
        res
    }

    pub async fn create_notifier(&self, client: Arc<QbClient>, tx: Sender<CheckType>) {
        let hash = self.torrent_hash.clone();
        if self.status {
            let res = Self::get_name(&client, &hash).await;
            if let Some(name) = res {
                tokio::spawn(async move {
                    while Self::check_is_completed(&client, &hash, &name).await.is_err() {
                        sleep(Duration::from_secs(1)).await;
                    };
                    if tx.send(Completed(name)).await.is_err() {
                        error!("Failed to send 'completed' status into channel")
                    }
                });
            } else {
                error!("Failed to get torrent name");
            }
        }
    }

    pub async fn send_link(mut self, arc_client: Arc<QbClient>, link: &str) -> Result<Self> {
        let client = arc_client.deref();
        let list_before = Self::get_hashes(client).await;
        let send_resp = Self::inner_send(client, link).await?;
        self.status =
            send_resp.status().is_success() && self.check_added(client, list_before).await.is_ok();
        Ok(self)
    }
}

impl QbCommandAction for QDownloadAction {
    fn action_result_to_string(&self) -> String {
        if self.status { "OK" } else { "FAIL" }.to_string()
    }
}
