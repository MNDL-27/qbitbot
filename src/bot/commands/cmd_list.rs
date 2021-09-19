use serde::Serialize;

#[derive(Serialize)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct QTag {
    pub tags: String,
}

#[derive(Serialize)]
pub struct QbList {
    pub sort: String,
}

#[derive(Serialize)]
pub struct QDownload {
    pub urls: String,
    pub tags: String,
}

#[derive(Serialize)]
pub struct QGetProperties {
    pub hash: String
}

#[derive(Serialize)]
pub struct QPause {
    pub hashes: String,
}

#[derive(Serialize)]
pub struct QResume {
    pub hashes: String,
}
