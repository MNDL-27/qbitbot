use serde::Serialize;

#[derive(Serialize)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct QbList {
    pub sort: String
}

#[derive(Serialize)]
pub struct QDownload {
    pub urls: String
}

#[derive(Serialize)]
pub struct QGetProperties {
}
