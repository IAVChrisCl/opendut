use anyhow::anyhow;
use config::Config;
use oauth2::{ClientId as OAuthClientId, ClientSecret as OAuthClientSecret, Scope as OAuthScope};
use pem::Pem;
use serde::{Deserialize, Serialize};
use shadow_rs::formatcp;
use tracing::debug;
use url::Url;

use opendut_carl_api::carl::auth::auth_config::OidcIdentityProviderConfig;
use opendut_carl_api::carl::auth::reqwest_client::PemFromConfig;
use opendut_types::util::net::{ClientId, ClientSecret};

use crate::auth::oidc_client_manager::CommonPeerCredentials;
use crate::settings::CarlUrl;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CarlIdentityProviderConfig {
    // TODO: consolidate/merge with authentication manager and OidcIdentityProviderConfig
    pub(crate) client_id: OAuthClientId,
    pub(crate) client_secret: OAuthClientSecret,
    pub(crate) issuer_url: Url,
    pub(crate) issuer_remote_url: Url,
    pub(crate) issuer_ca: Pem,
    pub(crate) scopes: Vec<OAuthScope>,
    pub(crate) peer_credentials: Option<CommonPeerCredentials>,
    pub(crate) carl_url: CarlUrl,
}

impl TryFrom<&Config> for CarlIdentityProviderConfig {
    type Error = anyhow::Error;

    fn try_from(config: &Config) -> anyhow::Result<Self> {
        let client_id = config.get_string(CarlIdentityProviderConfig::CLIENT_ID)
            .map_err(|error| anyhow!("Failed to find configuration for `{}`. {}", CarlIdentityProviderConfig::CLIENT_ID, error))?;
        let client_secret = config.get_string(CarlIdentityProviderConfig::CLIENT_SECRET)
            .map_err(|error| anyhow!("Failed to find configuration for `{}`. {}", CarlIdentityProviderConfig::CLIENT_SECRET, error))?;
        let issuer = config.get_string(CarlIdentityProviderConfig::ISSUER_URL)
            .map_err(|error| anyhow!("Failed to find configuration for `{}`. {}", CarlIdentityProviderConfig::ISSUER_URL, error))?;
        let issuer_remote = config.get_string(CarlIdentityProviderConfig::ISSUER_REMOTE_URL)
            .map_err(|error| anyhow!("Failed to find configuration for `{}`. {}", CarlIdentityProviderConfig::ISSUER_REMOTE_URL, error))?;

        let peer_id = config.get_string(CarlIdentityProviderConfig::COMMON_PEER_ID).ok();
        let peer_secret = config.get_string(CarlIdentityProviderConfig::COMMON_PEER_SECRET).ok();

        let peer_credentials = match (peer_id, peer_secret) {
            (Some(id), Some(secret)) => {
                debug!("Using defined common peer credentials for all peers with id='{}'", id);
                Some(CommonPeerCredentials {
                    client_id: ClientId(id),
                    client_secret: ClientSecret(secret),
                })
            }
            _ => None
        };

        let issuer_url = Url::parse(&issuer)
            .map_err(|error| anyhow!("Failed to parse issuer URL: {}", error))?;
        let issuer_remote_url = Url::parse(&issuer_remote)
            .map_err(|error| anyhow!("Failed to parse issuer remote URL: {}", error))?;

        let raw_scopes = config.get_string(CarlIdentityProviderConfig::SCOPES).unwrap_or_default();
        let issuer_ca = futures::executor::block_on(Pem::from_config_path("network.tls.ca", config))
            .map_err(|error| anyhow!("Failed to load issuer CA: {}", error))?;

        let carl_url = CarlUrl::try_from(config)?;

        Ok(Self {
            client_id: OAuthClientId::new(client_id.clone()),
            client_secret: OAuthClientSecret::new(client_secret),
            issuer_url,
            issuer_remote_url,
            issuer_ca,
            scopes: OidcIdentityProviderConfig::parse_scopes(&client_id, raw_scopes),
            peer_credentials,
            carl_url,
        })
    }
}

pub(crate) const CARL_OIDC_CONFIG_PREFIX: &str = "network.oidc.client";

impl CarlIdentityProviderConfig {
    const CLIENT_ID: &'static str = formatcp!("{CARL_OIDC_CONFIG_PREFIX}.id");
    const CLIENT_SECRET: &'static str = formatcp!("{CARL_OIDC_CONFIG_PREFIX}.secret");
    const COMMON_PEER_ID: &'static str = formatcp!("{CARL_OIDC_CONFIG_PREFIX}.peer.id");
    const COMMON_PEER_SECRET: &'static str = formatcp!("{CARL_OIDC_CONFIG_PREFIX}.peer.secret");
    const ISSUER_URL: &'static str = formatcp!("{CARL_OIDC_CONFIG_PREFIX}.issuer.url");
    const ISSUER_REMOTE_URL: &'static str = formatcp!("{CARL_OIDC_CONFIG_PREFIX}.issuer.remote.url");
    const SCOPES: &'static str = formatcp!("{CARL_OIDC_CONFIG_PREFIX}.scopes");
}
