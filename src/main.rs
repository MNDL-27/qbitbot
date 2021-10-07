mod bot;

use std::sync::Arc;

use bot::qbot::QbitBot;
use futures_util::stream::StreamExt;

#[tokio::main]
async fn main() {
    let qbot = QbitBot::new().await;
    println!("QbitBot launched");
    let mut updates_stream = Box::pin(qbot.rbot.incoming_updates(None, None));
    let qbot_arc = Arc::new(qbot);
    loop {
        match updates_stream.next().await.transpose() {
            Ok(update_opt) => {
                if let Some(update) = update_opt {
                    let res = QbitBot::proccess_message(qbot_arc.clone(), update);
                    println!("{:#?}", res)
                } else {
                    println!("Failed to parse message from Telegram")
                }
            }
            Err(err) => println!("{:#?}", err),
        };
    }
}
