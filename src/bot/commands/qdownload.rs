use std::collections::HashSet;

use crate::bot::{qb_client::QbClient, qbot::MessageWrapper};
use anyhow::Result;
use reqwest::Response;
use tokio::sync::mpsc::Sender;

use super::{cmd_list::QDownload, qlist::QListAction, QbCommandAction};

pub struct QDownloadAction {
    status: bool,
    validate: bool,
    notify: bool,
    torrent_hash: String,
    tg_tx: Sender<MessageWrapper>,
}

impl QDownloadAction {
    pub fn new(validate: bool, notify: bool, tg_tx: Sender<MessageWrapper>) -> Self {
        QDownloadAction {
            status: false,
            validate,
            notify,
            torrent_hash: "".to_string(),
            tg_tx,
        }
    }

    async fn check_added(
        &mut self,
        client: &QbClient,
        list_before: Option<HashSet<String>>,
    ) -> bool {
        if let Some(hashes) = list_before {
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
                "/command/download",
                QDownload {
                    urls: link.to_string(),
                },
            )
            .await?;
        Ok(resp)
    }

    async fn get_hashes(client: &QbClient) -> Option<HashSet<String>> {
        let hashes = QListAction::new()
            .get_raw(client, "")
            .await
            .ok()?
            .as_array()?
            .iter()
            .filter_map(|item| serde_json::from_value(item.get("hash").cloned()?).ok())
            .collect();
        Some(hashes)
    }

    fn create_notify(&self) {
        let tg_tx = self.tg_tx.clone();
        let hash = self.torrent_hash.clone();
        tokio::spawn(async move {
            // TODO: process torrent completed
            let msg = MessageWrapper {
                text: format!("{} is done", hash),
                parse_mode: None,
            };
            tg_tx.send(msg).await;
        });
    }

    pub async fn send_link(mut self, client: &QbClient, link: &str) -> Result<Self> {
        if self.validate {
            let list_before = Self::get_hashes(client).await;
            let send_resp = Self::inner_send(client, link).await?;
            self.status =
                send_resp.status().is_success() && self.check_added(client, list_before).await;
            if self.status && self.notify {
                self.create_notify()
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
