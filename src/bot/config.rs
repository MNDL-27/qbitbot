use std::collections::HashSet;

macro_rules! get_dotenv_var_or_panic {
    ($var:literal) => {
        dotenv::var($var).unwrap_or_else(|_| panic!("Please provide {} in the .env file", $var))
    };
}

#[derive(Debug, Clone)]
pub struct QbConfig {
    pub location: String,
    pub user: String,
    pub password: String,
    pub admins: HashSet<String>,
    pub log_level: String,
    pub token: String,
}

impl QbConfig {
    pub fn load_path(path: &str) -> Self {
        dotenv::from_filename(path).unwrap_or_else(|_| panic!("{} config file was not found!", path));
        QbConfig {
            location: get_dotenv_var_or_panic!("QBLOCATION"),
            user: get_dotenv_var_or_panic!("QBUSER"),
            password: get_dotenv_var_or_panic!("QBPASS"),
            admins: get_dotenv_var_or_panic!("ADMIN")
                .split(' ')
                .map(String::from)
                .collect(),
            token: get_dotenv_var_or_panic!("TOKEN"),
            log_level: dotenv::var("LOG_LEVEL").unwrap_or_else(|_| String::from("info")),
        }
    }

    pub fn load() -> Self {
        Self::load_path(".env")
    }
}
