use anyhow::Context;
use config::Config;
use serde::{Deserialize, Serialize};
use url::{ParseError, Url};

use opendut_util::settings::{LoadedConfig, LoadError};
use crate::resources::Id;

pub fn load_with_overrides(overrides: config::Config) -> Result<LoadedConfig, LoadError> {
    let carl_config_hide_secrets_override = config::Config::builder()
        .set_override("vpn.netbird.auth.secret", "redacted")?
        .set_override("network.oidc.client.secret", "redacted")?
        .build()?;

    opendut_util::settings::load_config("carl", include_str!("../carl.toml"), config::FileFormat::Toml, overrides, carl_config_hide_secrets_override)
}

#[cfg(test)]
pub fn load_defaults() -> Result<LoadedConfig, LoadError> {
    load_with_overrides(config::Config::default())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CarlUrl(Url);

impl TryFrom<&Config> for CarlUrl {
    type Error = anyhow::Error;

    fn try_from(config: &Config) -> Result<Self, Self::Error> {
        let carl_url = {
            let host = config.get_string("network.remote.host").expect("Configuration value for 'network.remote.host' should be set.");
            let port = config.get_int("network.remote.port").expect("Configuration value for 'network.remote.port' should be set.");
            Url::parse(&format!("https://{host}:{port}"))
                .context(format!("Could not create CARL URL from given host '{host}' and {port}."))?
        };
        Ok(Self(carl_url))
    }
}

impl CarlUrl {
    pub fn new(url: Url) -> Self { Self(url) }
    pub fn value(&self) -> Url {
        self.0.clone()
    }
    pub fn resource_url(&self, resource_id: Id) -> Result<Url, ParseError> {
        let path = format!("/resources/{}", resource_id.value());
        self.0.join(&path)
    }
}
