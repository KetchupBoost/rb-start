use std::env;

use postgres::{Client, Error, NoTls};

struct DatabaseConfig {
    host: String,
    user: String,
    password: String,
    dbname: String 
}

impl DatabaseConfig {
    fn from_environment() -> Self {
        Self {
            host: env::var("DATABASE_HOST").unwrap(),
            user: env::var("DATABASE_USER").unwrap(),
            password: env::var("DATABASE_PASSWORD").unwrap(),
            dbname: env::var("DATABASE_NAME").unwrap(),
        }
    }

    fn config_to_string(&self) -> Result<String, Error> {
        let config = format!(
            "host={} user={} password={} dbname={}",
            &self.host, &self.user, &self.password, &self.dbname
        );
        
        Ok(config)
    }
}

pub struct Database {
    pub conn: Client,
}

impl Database {
    pub fn new() -> Result<Database, Error> {
        let db_config = DatabaseConfig::from_environment();

        let pg_config = db_config.config_to_string()?;

        let conn = Client::connect(pg_config.as_str(), NoTls)?;

        Ok(Self { conn })
    }
}
