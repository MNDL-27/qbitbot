use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use fure::backoff::fixed;
use fure::policies::{attempts, backoff};
use futures::FutureExt;
use reqwest::Response;
use tokio::time::Duration;

use crate::bot::{qb_chat::NotifyClosure, qb_client::QbClient, qbot::MessageWrapper, TAG_NAME};

use super::{cmd_list::QDownload, list::QListAction, QbCommandAction};

pub struct QDownloadAction {
    status: bool,
    validate: bool,
    notify: bool,
    torrent_hash: String,
    torrent_name: String,
    pub checking_closure: Option<NotifyClosure>,
}

enum CheckStatus {
    Done,
    InProgress,
    Fail,
}

impl QDownloadAction {
    pub fn new(validate: bool, notify: bool) -> Self {
        QDownloadAction {
            status: false,
            validate,
            notify,
            torrent_hash: "".to_string(),
            torrent_name: "".to_string(),
            checking_closure: None,
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

    async fn check_is_complete(client: &QbClient, hash: &str) -> CheckStatus {
        if let Ok(props) = client.get_properties(hash.to_string()).await {
            let completion_date_opt = move || -> Option<i64> {
                let obj = props.as_object()?;
                obj.get("completion_date")?.as_i64()
            }();
            if let Some(completion_date) = completion_date_opt {
                if completion_date != -1 {
                    CheckStatus::Done
                } else {
                    CheckStatus::InProgress
                }
            } else {
                CheckStatus::Fail
            }
        } else {
            CheckStatus::Fail
        }
    }

    async fn notify(client: Arc<QbClient>, hash: String, name: String) -> Result<MessageWrapper> {
        let res = match Self::check_is_complete(&client, &hash).await {
            CheckStatus::Done => Ok(MessageWrapper {
                text: format!("{} is done", name),
                parse_mode: None,
            }),
            CheckStatus::Fail => Err(anyhow!("Failed to get torrent status")),
            CheckStatus::InProgress => Err(anyhow!("Torrent in progress")),
        };
        debug!("Checked {} for completion", name);
        res
    }

    pub async fn send_link(mut self, arc_client: Arc<QbClient>, link: &str) -> Result<Self> {
        let client = arc_client.deref();
        if self.validate {
            let list_before = Self::get_hashes(client).await;
            let send_resp = Self::inner_send(client, link).await?;
            self.status = send_resp.status().is_success()
                && self.check_added(client, list_before).await.is_ok();
            if self.status && self.notify {
                self.torrent_name = Self::get_name(client, &self.torrent_hash)
                    .await
                    .ok_or_else(|| anyhow!("Failed to get torrent name"))?;
                let (r_hash, r_name) = (self.torrent_hash.clone(), self.torrent_name.clone());
                let closure: NotifyClosure = Box::new(move |arc_client| {
                    let (hash, name) = (r_hash.clone(), r_name.clone());
                    async move { Self::notify(arc_client, hash, name).await }.boxed()
                });
                self.checking_closure = Some(closure);
            }
        } else {
            let send_resp = Self::inner_send(client, link).await?;
            self.status = send_resp.status().is_success();
        };
        Ok(self)
    }
}

impl QbCommandAction for QDownloadAction {
    fn action_result_to_string(&self) -> String {
        if self.status { "OK" } else { "FAIL" }.to_string()
    }
}
