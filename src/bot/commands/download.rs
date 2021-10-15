use std::collections::HashSet;

use crate::bot::{qb_chat::NotifyClosure, qb_client::QbClient, qbot::MessageWrapper, TAG_NAME};
use anyhow::{anyhow, Result};
use reqwest::Response;
use tokio::time::{sleep, Duration};

use super::{cmd_list::QDownload, list::QListAction, BoxedCommand, QbCommandAction};

pub struct QDownloadAction {
    status: bool,
    validate: bool,
    notify: bool,
    torrent_hash: String,
    torrent_name: String,
}

enum CheckStatus {
    Done,
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
        }
    }

    async fn check_added(
        &mut self,
        client: &QbClient,
        list_before: Option<HashSet<String>>,
    ) -> bool {
        if let Some(hashes) = list_before {
            // give some time to start downloading
            sleep(Duration::from_millis(1000)).await;
            let list_after = Self::get_hashes(client).await.unwrap_or_default();
            let diff: HashSet<String> = list_after.difference(&hashes).cloned().collect();
            // TODO: fix this barely good check
            if diff.len() == 1 {
                self.torrent_hash = diff.iter().cloned().next().unwrap();
                true
            } else {
                false
            }
        } else {
            false
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
                    CheckStatus::Fail
                }
            } else {
                CheckStatus::Fail
            }
        } else {
            CheckStatus::Fail
        }
    }

    async fn notify(client: &QbClient, hash: String, name: String) -> Result<MessageWrapper> {
        let res = match Self::check_is_complete(&client, &hash).await {
            CheckStatus::Done => Ok(MessageWrapper {
                text: format!("{} is done", name),
                parse_mode: None,
            }),
            CheckStatus::Fail => {
                error!("Failed to get torrent status: {}", name);
                Err(anyhow!("Failed to get torrent status"))
            }
        };
        info!("Created notify loop for {}", name);
        res
    }

    pub async fn send_link(mut self, client: & QbClient, link: &str) -> Result<Self> {
        if self.validate {
            let list_before = Self::get_hashes(client).await;
            let send_resp = Self::inner_send(client, link).await?;
            self.status =
                send_resp.status().is_success() && self.check_added(client, list_before).await;
            if self.status && self.notify {
                self.torrent_name = Self::get_name(client, &self.torrent_hash)
                    .await
                    .ok_or_else(|| anyhow!("Failed to get torrent name"))?;
                // let (hash, name) = (self.torrent_hash.clone(), self.torrent_name.clone());
                // let a: NotifyClosure = Box::new(|| {
                //     Box::pin(async {
                //         Self::notify(client, hash, name)
                //             .await
                //     })
                // });
            }
        } else {
            let send_resp = Self::inner_send(client, link).await?;
            self.status = send_resp.status().is_success();
        };
        Ok(self)
    }
}

impl QbCommandAction for QDownloadAction {
    fn name(&self) -> String {
        "/download".to_string()
    }

    fn action_result_to_string(&self) -> String {
        if self.status { "OK" } else { "FAIL" }.to_string()
    }
}

impl BoxedCommand for QDownloadAction {}
