use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{HeaderMap, ORIGIN},
    Client, Response,
};
use serde::Serialize;
use serde_json::Value;

use crate::bot::commands::{
    cmd_list::{QHelp, UnknownCommand},
    qlist::QListAction,
    QbCommandAction,
};

use super::{commands::cmd_list::Login, config::QbConfig, qbot::RbotParseMode};

#[derive(Debug)]
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

    pub async fn qsend<T: Serialize>(&self, location: &str, action: T) -> Result<Response> {
        let mut api_loc = self.config.location.clone();
        api_loc.push_str(location);
        let resp = self
            .client
            .post(api_loc)
            .form(&action)
            .send()
            .await
            .with_context(|| "Failed to send request")?;

        if resp.status().is_success() {
            Ok(resp)
        } else {
            Err(anyhow!(format!(
                "Command failed. Status code: {}",
                resp.status()
            )))
        }
    }

    pub async fn qsend_json<T: Serialize>(&self, location: &str, action: T) -> Result<Value> {
        Ok(self.qsend(location, action).await?.json().await?)
    }

    pub async fn login(&self) -> Result<()> {
        let login = Login {
            username: self.config.user.clone(),
            password: self.config.password.clone(),
        };
        self.qsend("login", login).await?;
        Ok(())
    }

    pub async fn do_cmd(&self, text: &str) -> Result<(String, RbotParseMode)> {
        let cmd_result: Box<dyn QbCommandAction> = match text {
            "/help" => Box::new(QHelp {}),
            "/list" => Box::new(QListAction::new().get(&self, "").await?),
            _ => Box::new(UnknownCommand {}),
        };

        let prepared = match cmd_result.parse_mode() {
            Some(rutebot::requests::ParseMode::Html) => {
                format!("<pre>{}</pre>", cmd_result.convert_to_string())
            }
            _ => cmd_result.convert_to_string(),
        };
        
        // Telegram message size is limited by 4096 characters
        let cut_msg: String = prepared.chars().take(4096).collect();

        Ok((cut_msg, cmd_result.parse_mode()))
    }
}
