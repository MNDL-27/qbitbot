use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{
    Client,
    header::{HeaderMap, ORIGIN}, Response,
};
use serde::Serialize;

use super::{
    commands::{
        cmd_list::{Login, QTag},
        list::QListAction,
    },
    config::QbConfig,
    TAG_NAME,
};

#[derive(Debug, Clone)]
pub struct QbClient {
    pub config: QbConfig,
    client: Client,
    cached_list: Option<QListAction>,
}

impl QbClient {
    pub async fn new(config: &QbConfig) -> Self {
        let headers = Self::gen_headers(config.location.clone());
        let qbclient = QbClient {
            client: Self::build_client(headers),
            cached_list: None,
            config: config.to_owned(),
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
            .timeout(Duration::from_secs(10))
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

    pub async fn get_cached_list(&mut self) -> Result<QListAction> {
        if self.cached_list.is_some() {
            let mut cached_list = self.cached_list.to_owned().unwrap();
            cached_list.check_and_update(self).await?;
            self.cached_list = Some(cached_list.clone());
            Ok(cached_list)
        } else {
            let cached_list = QListAction::new(self).await?;
            self.cached_list = Some(cached_list.to_owned());
            Ok(cached_list)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct EmptyAction{
        test: i32
    }

    #[tokio::test]
    async fn test_client_start() {
        let conf = QbConfig::load_path("tests/.env_tests");
        let client = QbClient::new(&conf).await;
        client.qpost("/app/version", EmptyAction{test: 1}).await.unwrap();
    }
}
