use std::collections::HashSet;

use crate::bot::{qb_client::QbClient, qbot::MessageWrapper};
use anyhow::{anyhow, Result};
use reqwest::Response;
use tokio::{
    sync::mpsc::Sender,
    time::{sleep, Duration},
};

use super::{cmd_list::QDownload, qlist::QListAction, QbCommandAction};

pub struct QDownloadAction {
    status: bool,
    validate: bool,
    notify: bool,
    torrent_hash: String,
    torrent_name: String,
    tg_tx: Sender<MessageWrapper>,
}

impl QDownloadAction {
    pub fn new(validate: bool, notify: bool, tg_tx: Sender<MessageWrapper>) -> Self {
        QDownloadAction {
            status: false,
            validate,
            notify,
            torrent_hash: "".to_string(),
            torrent_name: "".to_string(),
            tg_tx,
        }
    }

    async fn check_added(
        &mut self,
        client: &QbClient,
        list_before: Option<HashSet<String>>,
    ) -> bool {
        if let Some(hashes) = list_before {
            // give some time to start downloading
            sleep(Duration::from_millis(500)).await;
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
            .get(client)
            .await
            .ok()?
            .as_array()?
            .iter()
            .filter_map(|item| serde_json::from_value(item.get("hash").cloned()?).ok())
            .collect();
        Some(hashes)
    }

    async fn get_name(client: &QbClient, hash: &str) -> Option<String> {
        let list_info = QListAction::new().get(client).await.ok()?;
        println!("{:#?}", list_info);
        let name = list_info
            .as_array()?
            .iter().find(|item| item.get("hash").unwrap() == hash)?
            .as_object()?
            .get("name")?
            .to_string();
        Some(name)
    }

    
    async fn check_is_complete(client: &QbClient, hash: &str) -> Option<()> {
        // TODO: process accedential torrent remove
        let props = client.get_properties(hash).await.ok()?;
        let obj = props.as_object()?;
        let eta = obj.get("eta")?.as_i64()?;
        let total_dowloaded = obj.get("total_downloaded")?.as_i64()?;
        println!("eta: {}, total: {}", eta, total_dowloaded);
        if total_dowloaded > 0 && eta == 8640000 {
            Some(())
        } else {
            None
        }
    }

    fn create_notify(&self, client: &QbClient) {
        let tg_tx = self.tg_tx.clone();
        let hash = self.torrent_hash.clone();
        let name = self.torrent_name.clone();
        let my_client = (*client).clone();
        tokio::spawn(async move {
            while Self::check_is_complete(&my_client, &hash).await.is_none() {
                sleep(Duration::from_millis(1000)).await
            }

            let msg = MessageWrapper {
                text: format!("{} is done", name),
                parse_mode: None,
            };
            tg_tx.send(msg).await;
        });
        println!("Created notify loop for {}", self.torrent_name);
    }

    pub async fn send_link(mut self, client: &QbClient, link: &str) -> Result<Self> {
        if self.validate {
            let list_before = Self::get_hashes(client).await;
            let send_resp = Self::inner_send(client, link).await?;
            self.status =
                send_resp.status().is_success() && self.check_added(client, list_before).await;
            if self.status && self.notify {
                self.torrent_name = Self::get_name(client, &self.torrent_hash)
                    .await
                    .ok_or_else(|| anyhow!("Failed to get torrent name"))?;
                self.create_notify(client)
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
