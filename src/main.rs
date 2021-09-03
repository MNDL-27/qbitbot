mod bot;

use anyhow::Result;
use bot::qbot::QbitBot;
use futures_util::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut qbot = QbitBot::new().await;
    println!("QbitBot launched");
    let mut updates_stream = Box::pin(qbot.rbot.incoming_updates(None, None));
    while let Some(update) = updates_stream.next().await.transpose()? {
        let res = qbot.proccess_message(update).await;
        println!("{:#?}", res)
    }
    Ok(())
}
