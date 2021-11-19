use std::collections::HashSet;

use anyhow::{anyhow, Result};
use fure::backoff::fixed;
use fure::policies::{attempts, backoff};
use reqwest::Response;
use tokio::sync::mpsc::Sender;
use tokio::time::{sleep, Duration};

use crate::bot::notifier::CheckType;
use crate::bot::notifier::CheckType::Completed;
use crate::bot::{qb_client::QbClient, TAG_NAME};

use super::{cmd_list::QDownload, list::QListAction, QbCommandAction};

pub struct QDownloadAction {
    status: bool,
    torrent_hash: String,
}

impl QDownloadAction {
    pub fn new() -> Self {
        QDownloadAction {
            status: false,
            torrent_hash: "".to_string(),
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
        let hashes = QListAction::get(client)
            .await
            .ok()?
            .as_array()?
            .iter()
            .filter_map(|item| serde_json::from_value(item.get("hash").cloned()?).ok())
            .collect();
        Some(hashes)
    }

    pub async fn get_name(client: &mut QbClient, hash: &str) -> Option<String> {
        client
            .get_cached_list()
            .await
            .unwrap()
            .get_records()
            .iter()
            .find_map(|item| {
                let item_hash = item.get_hash();
                if item.get_hash() == hash {
                    Some(item_hash)
                } else {
                    None
                }
            })
    }

    async fn check_is_completed(client: &QbClient, hash: &str, name: &str) -> Result<String> {
        let res = if let Ok(props) = QListAction::get_properties(client, hash.to_string()).await {
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

    pub async fn create_notifier(&self, client: &mut QbClient, tx: Sender<CheckType>) {
        let hash = self.torrent_hash.clone();
        if self.status {
            let res = Self::get_name(client, &hash).await;
            if let Some(name) = res {
                let my_client = client.clone();
                tokio::spawn(async move {
                    while Self::check_is_completed(&my_client, &hash, &name)
                        .await
                        .is_err()
                    {
                        sleep(Duration::from_secs(1)).await;
                    }
                    if tx.send(Completed(name)).await.is_err() {
                        error!("Failed to send 'completed' status into channel")
                    }
                });
            } else {
                error!("Failed to get torrent name");
            }
        }
    }

    pub async fn send_link(mut self, client: &QbClient, link: &str) -> Result<Self> {
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
