use serde::Serialize;

use super::QbCommandAction;

pub struct QHelp {}

impl QbCommandAction for QHelp {
    fn name(&self) -> String {
        "/help".to_string()
    }

    fn convert_to_string(&self) -> String {
        "Help is here".to_string()
    }
}

pub struct UnknownCommand {}

impl QbCommandAction for UnknownCommand {
    fn name(&self) -> String {
        todo!()
    }

    fn convert_to_string(&self) -> String {
        "Unknown command".to_string()
    }
}

#[derive(Serialize)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct QbList {
    pub filter: String,
}
