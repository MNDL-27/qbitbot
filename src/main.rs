mod bot;

use bot::qbot::QbitBot;
use futures_util::stream::StreamExt;

#[tokio::main]
async fn main() {
    let mut qbot = QbitBot::new().await;
    println!("QbitBot launched");
    let mut updates_stream = Box::pin(qbot.rbot.incoming_updates(None, None));
    loop {
        match updates_stream.next().await.transpose() {
            Ok(update_opt) => {
                if let Some(update) = update_opt {
                    let res = qbot.proccess_message(update).await;
                    println!("{:#?}", res)
                } else {
                    println!("None")
                }
            }
            Err(err) => println!("{:#?}", err),
        };
    }
}
