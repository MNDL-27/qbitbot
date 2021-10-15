use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{HeaderMap, ORIGIN},
    Client, Response,
};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::mpsc::Sender;

use crate::bot::commands::{
    cmd_list::QGetProperties, list::QListAction, pause::QPauseAction, resume::QResumeAction,
    QbCommandAction,
};

use super::{
    commands::{
        cmd_list::{Login, QTag},
        download::QDownloadAction,
        simple::{QHelp, QStart, UnknownCommand},
        BoxedCommand,
    },
    config::QbConfig,
    qbot::{MessageWrapper, RbotParseMode},
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
        qbclient.create_tag().await.unwrap();
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

    async fn select_command_with_id(
        &self,
        command: &str,
        id_str: &str,
    ) -> Result<Box<dyn QbCommandAction>> {
        let id = id_str.parse::<usize>()?;
        let action: Box<dyn QbCommandAction> = match command {
            "/select" => todo!(),
            "/pause" => QPauseAction { status: false }.act(self, id).await?.boxed(),
            "/resume" => QResumeAction { status: false }.act(self, id).await?.boxed(),
            _ => UnknownCommand {}.boxed(), // this could never happen
        };
        Ok(action)
    }

    // pub async fn do_cmd(
    //     &self,
    //     text: &str,
    //     tg_tx: Sender<MessageWrapper>,
    // ) -> Result<(String, RbotParseMode)> {
    //     let tokens = text.split(' ').collect::<Vec<_>>();
    //     let cmd_result: Box<dyn QbCommandAction> = match tokens.as_slice() {
    //         ["/help"] => QHelp {}.boxed(),
    //         ["/start"] => QStart {}.boxed(),
    //         ["/list"] => QListAction::new().get_formatted(self).await?.boxed(),
    //         [cmd @ ("/select" | "/pause" | "/resume"), id_str]
    //             if id_str.parse::<usize>().is_ok() =>
    //         {
    //             self.select_command_with_id(cmd, id_str).await?
    //         }
    //         ["/download", link] => QDownloadAction::new(true, true, tg_tx)
    //             .send_link(self, link)
    //             .await?
    //             .boxed(),
    //         _ => UnknownCommand {}.boxed(),
    //     };

    //     // TODO: escape HTML and Markdown special chars
    //     let prepared = match cmd_result.parse_mode() {
    //         Some(rutebot::requests::ParseMode::Html) => {
    //             format!("<pre>{}</pre>", cmd_result.action_result_to_string())
    //         }
    //         _ => cmd_result.action_result_to_string(),
    //     };

    //     // Telegram message size is limited by 4096 characters
    //     let cut_msg: String = prepared.chars().take(4096).collect();
    //     // TODO: split message by parts if it does not fit

    //     Ok((cut_msg, cmd_result.parse_mode()))
    // }
}
