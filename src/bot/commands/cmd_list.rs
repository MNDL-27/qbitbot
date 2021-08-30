use rutebot::requests::ParseMode;
use serde::Serialize;

use crate::bot::qbot::RbotParseMode;

use super::QbCommandAction;

#[derive(Serialize)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct QbList {
    pub filter: String,
}
