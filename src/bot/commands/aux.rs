use crate::bot::commands::download::QDownloadAction;
use crate::bot::commands::list::QListAction;
use crate::bot::qb_client::QbClient;

pub async fn id_to_hash(client: &QbClient, id: usize) -> Option<String> {
    let array = QListAction::get(client).await.ok()?.as_array()?.to_owned();
    let (_, item) = array.iter().enumerate().nth(id)?;
    let hash = item.as_object()?.get("hash")?.as_str()?.to_string();
    Some(hash)
}

pub async fn get_name_by_id(client: &QbClient, id: usize) -> Option<String> {
    let hash = id_to_hash(client, id).await?;
    QDownloadAction::get_name(client, &hash).await
}