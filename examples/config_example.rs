use serde::{Deserialize, Serialize};
use grafton_config::{TokenExpandingConfig, Error};

#[derive(Debug, Serialize, Deserialize)]
struct Server {
    host: String,
    port: u16,
    database_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    server: Server,
}

impl TokenExpandingConfig for AppConfig {}

fn main() -> Result<(), Error> {
    let config = grafton_config::load_config_from_dir::<AppConfig>("examples/config")?;
    println!("Database URL: {}", config.server.database_url);
    Ok(())
}
