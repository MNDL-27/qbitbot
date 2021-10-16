use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{HeaderMap, ORIGIN},
    Client, Response,
};
use serde::Serialize;
use serde_json::Value;

use crate::bot::commands::cmd_list::QGetProperties;

use super::{
    commands::cmd_list::{Login, QTag},
    config::QbConfig,
    TAG_NAME,
};

#[derive(Debug, Clone)]
pub struct QbClient {
    pub config: QbConfig,
    client: Client,
}

impl QbClient {
    pub async fn new() -> Self {
        let config = QbConfig::load();
        let headers = Self::gen_headers(config.location.clone());
        let qbclient = QbClient {
            client: Self::build_client(headers),
            config,
        };
        qbclient.login().await.unwrap();
        if qbclient.create_tag().await.is_err() {
            error!("Failed to create tag for Qbitbot. Probably your qbittorrent version doesn't support it")
        };
        qbclient
    }

    fn gen_headers(origin: String) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, origin.parse().unwrap());
        headers
    }

    fn build_client(headers: HeaderMap) -> Client {
        Client::builder()
            .cookie_store(true)
            .default_headers(headers)
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap()
    }

    pub async fn qpost<T: Serialize>(&self, location: &str, action: T) -> Result<Response> {
        let mut api_loc = self.config.location.clone();
        api_loc.push_str("/api/v2");
        api_loc.push_str(location);
        let resp = self
            .client
            .post(api_loc)
            .form(&action)
            .send()
            .await
            .with_context(|| "Failed to send POST request")?;

        if resp.status().is_success() {
            trace!("{:#?}", resp);
            Ok(resp)
        } else {
            Err(anyhow!(format!(
                "Command failed. Status code: {}",
                resp.status()
            )))
        }
    }

    pub async fn login(&self) -> Result<()> {
        let login = Login {
            username: self.config.user.clone(),
            password: self.config.password.clone(),
        };
        self.qpost("/auth/login", login).await?;
        Ok(())
    }

    async fn create_tag(&self) -> Result<()> {
        self.qpost(
            "/torrents/createTags",
            QTag {
                tags: TAG_NAME.to_string(),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn get_properties(&self, hash: String) -> Result<Value> {
        let value = self
            .qpost("/torrents/properties", QGetProperties { hash })
            .await?
            .json()
            .await?;
        Ok(value)
    }
}
