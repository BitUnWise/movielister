use color_eyre::{eyre::eyre, eyre::WrapErr, Result};
use serde::Deserialize;
use tokio::sync::OnceCell;

#[derive(Deserialize, Debug)]
pub struct Secrets {
    pub surreal_db_password: String,
}

const SECRETS_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/secrets.toml");
impl Secrets {
    fn load() -> Result<Secrets> {
        let secrets = std::fs::read_to_string(SECRETS_FILE)
            .wrap_err_with(|| eyre!("Failed to open secrets {SECRETS_FILE}"))?;
        Ok(toml::from_str(&secrets)?)
    }
}

static SECRETS: OnceCell<Secrets> = OnceCell::const_new();

pub async fn init_secrets() -> Result<()> {
    SECRETS
        .set(Secrets::load()?)
        .map_err(|_| eyre!("Failed to set secrets"))
}

pub async fn get_secrets() -> &'static Secrets {
    SECRETS.get().unwrap()
}
