use crate::bot::qb_client::QbClient;
use anyhow::Result;

use super::{cmd_list::QDownload, QbCommandAction};

pub struct QDownloadAction {
    status: bool,
}

impl QDownloadAction {
    pub fn new() -> Self {
        QDownloadAction { status: false }
    }

    pub async fn send_link(mut self, client: &QbClient, link: &str) -> Result<Self> {
        let resp = client
            .qsend(
                "/command/download",
                QDownload {
                    urls: link.to_string(),
                },
            )
            .await?;
        // No matter if successful or not server will return the 200 code
        // TODO: process successful addition
        self.status = resp.status().is_success();
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
