use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct QbConfig {
    pub location: String,
    pub user: String,
    pub password: String,
    pub admins: HashSet<String>,
}

impl QbConfig {
    pub fn load() -> Self {
        QbConfig {
            location: dotenv::var("QBLOCATION").expect(&format!(dotenv_err!(), "QBLOCATION")),
            user: dotenv::var("QBUSER").expect(&format!(dotenv_err!(), "QBUSER")),
            password: dotenv::var("QBPASS").expect(&format!(dotenv_err!(), "QBPASS")),
            admins: dotenv::var("ADMIN")
                .expect(&format!(dotenv_err!(), "ADMIN"))
                .split(' ')
                .map(String::from)
                .collect(),
        }
    }
}
