#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use futures_util::stream::StreamExt;
use rutebot::client::Rutebot;

use bot::qbot::QbitBot;

use crate::bot::config::QbConfig;

mod bot;

#[tokio::main]
async fn main() {
    let config = QbConfig::load();
    pretty_env_logger::formatted_builder()
        .parse_filters(&config.log_level)
        .init();
    let rbot = Rutebot::new(config.token.clone());
    let mut updates_stream = Box::pin(rbot.incoming_updates(None, None));
    let qbot = QbitBot::new(&config, rbot).await;
    info!("QbitBot launched");
    loop {
        match updates_stream.next().await.transpose() {
            Ok(update_opt) => {
                if let Some(update) = update_opt {
                    qbot.process_message(update).await;
                } else {
                    error!("Failed to parse message from Telegram")
                }
            }
            Err(err) => error!("{:#?}", err),
        };
    }
}
