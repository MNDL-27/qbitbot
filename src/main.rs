mod bot;

use anyhow::Result;
use bot::qbot::QbitBot;
use futures_util::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let qbot = QbitBot::new().await;
    println!("QbitBot launched");
    let mut updates_stream = Box::pin(qbot.rbot.incoming_updates(None, None));
    while let Some(update) = updates_stream.next().await.transpose()? {
        if let Some(reply) = qbot.proccess_message(update).await {
            tokio::spawn(
                async move {
                    let resp = reply.send().await;
                    println!("{:#?}", resp)
                }
            );
        }
    }
    Ok(())
}
