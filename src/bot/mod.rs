macro_rules! dotenv_err {
    () => {
        "Please provide {} in the .env file"
    };
}

pub const TAG_NAME: &str = "qbitbot";

pub mod commands;
pub mod config;
pub mod qb_client;
pub mod qbot;
mod qb_chat;
mod notifier;
