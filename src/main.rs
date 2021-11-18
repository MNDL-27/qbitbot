#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::sync::Arc;

use futures_util::stream::StreamExt;

use bot::qbot::QbitBot;

use crate::bot::config::QbConfig;

mod bot;

#[tokio::main]
async fn main() {
    let config = QbConfig::load();
    pretty_env_logger::formatted_builder()
        .parse_filters(&config.log_level)
        .init();
    let qbot = QbitBot::new(&config).await;
    info!("QbitBot launched");
    let mut updates_stream = Box::pin(qbot.rbot.incoming_updates(None, None));
    let qbot_arc = Arc::new(qbot);
    loop {
        match updates_stream.next().await.transpose() {
            Ok(update_opt) => {
                if let Some(update) = update_opt {
                    QbitBot::process_message(qbot_arc.clone(), update).await;
                } else {
                    error!("Failed to parse message from Telegram")
                }
            }
            Err(err) => error!("{:#?}", err),
        };
    }
}
