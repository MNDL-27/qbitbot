macro_rules! dotenv_err {
    () => {
        "Please provide {} in the .env file"
    };
}

pub mod qbot;
pub mod config;
pub mod commands;
pub mod qb_client;