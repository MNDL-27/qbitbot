use std::collections::HashSet;

use crate::bot::qb_client::QbClient;
use anyhow::Result;
use reqwest::Response;

use super::{cmd_list::QDownload, qlist::QListAction, QbCommandAction};

pub struct QDownloadAction {
    status: bool,
    validate: bool,
}

impl QDownloadAction {
    pub fn new(validate: bool) -> Self {
        QDownloadAction {
            status: false,
            validate,
        }
    }

    async fn check_added(client: &QbClient, list_before: Option<HashSet<String>>) -> bool {
        if let Some(hashes) = list_before {
            let list_after = Self::get_hashes(client).await.unwrap_or_default();
            let diff: HashSet<String> = list_after.difference(&hashes).cloned().collect();
            diff.len() == 1
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

    pub async fn send_link(mut self, client: &QbClient, link: &str) -> Result<Self> {
        if self.validate {
            let list_before = Self::get_hashes(client).await;
            let send_resp = Self::inner_send(client, link).await?;
            self.status =
                send_resp.status().is_success() && Self::check_added(client, list_before).await;
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
