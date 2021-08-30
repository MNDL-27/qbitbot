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
        client
            .qsend(
                "/command/download",
                QDownload {
                    urls: link.to_string(),
                },
            )
            .await.unwrap();
        // TODO: process successful torrent addition
        self.status = true;
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
