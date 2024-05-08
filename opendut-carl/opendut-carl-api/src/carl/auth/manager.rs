use std::sync::Arc;
use chrono::{NaiveDateTime, Utc};
use config::Config;
use oauth2::{AccessToken, AuthUrl, Scope, TokenResponse, TokenUrl};
use oauth2::basic::{BasicClient, BasicTokenResponse};
use tokio::sync::{RwLock, RwLockWriteGuard};
use tracing::debug;
use crate::carl::auth::error::parse_oauth_request_error;

use crate::carl::auth::reqwest_client::OidcReqwestClient;
use crate::carl::OidcIdentityProviderConfig;

#[derive(Debug)]
pub struct AuthenticationManager {
    oauth_client: BasicClient,
    reqwest_client: OidcReqwestClient,
    scopes: Vec<Scope>,

    state: RwLock<Option<TokenStorage>>,
}

#[derive(Debug, Clone)]
struct TokenStorage {
    pub access_token: AccessToken,
    pub expires_in: NaiveDateTime,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("FailedToGetToken: {message} cause: {cause}.")]
    FailedToGetToken { message: String, cause: String },
    #[error("ExpirationFieldMissing: {message}.")]
    ExpirationFieldMissing { message: String },
}

#[derive(thiserror::Error, Debug)]
pub enum AuthenticationManagerError {
    #[error("Failed to load OIDC configuration: '{message}'. Cause: '{cause}'")]
    Configuration { message: String, cause: Box<dyn std::error::Error + Send + Sync> },
}

pub struct Token {
    pub value: String,
}

pub type AuthenticationManagerRef = Arc<AuthenticationManager>;

pub const CONFIG_KEY_OIDC_ENABLED: &str = "network.oidc.enabled";

impl AuthenticationManager {
    pub async fn from_settings(settings: &Config) -> Result<Option<AuthenticationManagerRef>, AuthenticationManagerError> {
        let oidc_enabled = settings.get_bool(CONFIG_KEY_OIDC_ENABLED)
                .map_err(|cause| AuthenticationManagerError::Configuration { message: format!("No configuration found for {CONFIG_KEY_OIDC_ENABLED}."), cause: cause.into() })?;
        if oidc_enabled {
            let oidc_config = OidcIdentityProviderConfig::try_from(settings)
                .map_err(|cause| AuthenticationManagerError::Configuration {message: String::from("Failed to load OIDC configuration"), cause: cause.into() })?;
            debug!("OIDC configuration loaded: id={:?} issuer_url={:?}", oidc_config.client_id, oidc_config.issuer_url);
            let reqwest_client = OidcReqwestClient::from_config(settings).await
                .map_err(|cause| AuthenticationManagerError::Configuration {message: String::from("Failed to create reqwest client."), cause: cause.into() })?;
            let auth_manager = AuthenticationManager::from_oidc_config(oidc_config, reqwest_client).await?;

            Ok(Some(auth_manager))
        } else {
            debug!("OIDC is disabled.");
            Ok(None)
        }

    }

    async fn from_oidc_config(oidc_config: OidcIdentityProviderConfig, reqwest_client: OidcReqwestClient) -> Result<AuthenticationManagerRef, AuthenticationManagerError> {
        let auth_endpoint = oidc_config.issuer_url.join("protocol/openid-connect/auth")
            .map_err(|cause| AuthenticationManagerError::Configuration {message: String::from("Failed to derive authorization url from issuer url."), cause: cause.into() })?;
        let token_endpoint = oidc_config.issuer_url.join("protocol/openid-connect/token")
            .map_err(|cause| AuthenticationManagerError::Configuration {message: String::from("Failed to derive token url from issuer url."), cause: cause.into() })?;

        let oauth_client = BasicClient::new(
            oidc_config.client_id,
            Some(oidc_config.client_secret),
            AuthUrl::from_url(auth_endpoint),
            Some(TokenUrl::from_url(token_endpoint)),
        );

        let auth_manager = Self {
            oauth_client,
            reqwest_client,
            scopes: oidc_config.scopes,
            state: Default::default(),
        };
        Ok(Arc::new(auth_manager))
    }
    fn update_storage_token(response: &BasicTokenResponse, state: &mut RwLockWriteGuard<Option<TokenStorage>>) -> Result<Token, AuthError> {
        let access_token = response.access_token().clone();
        let expires_in = match response.expires_in() {
            None => {
                return Err(AuthError::ExpirationFieldMissing { message: "No expires_in in response.".to_string() });
            }
            Some(expiry_duration) => { Utc::now().naive_utc() + expiry_duration }
        };
        let _token_storage = state.insert(TokenStorage {
            access_token,
            expires_in,
        });
        Ok(Token { value: response.access_token().secret().to_string() })
    }

    async fn fetch_token(&self) -> Result<Token, AuthError> {
        let response = self.oauth_client.exchange_client_credentials()
            .add_scopes(self.scopes.clone())
            .request_async(|request| { self.reqwest_client.async_http_client(request) })
            .await
            .map_err(|error| {
                let error_string = parse_oauth_request_error(&error);

                AuthError::FailedToGetToken {
                    message: "Fetching authentication token failed!".to_string(),
                    cause: error_string,
                }
            })?;

        let mut state = self.state.write().await;

        Self::update_storage_token(&response, &mut state)?;

        Ok(Token { value: response.access_token().secret().to_string() })
    }

    pub async fn get_token(&self) -> Result<Token, AuthError> {
        let token_storage = self.state.read().await.clone();
        let access_token = match token_storage {
            None => {
                self.fetch_token().await?
            }
            Some(token) => {
                if Utc::now().naive_utc().lt(&token.expires_in) {
                    Token { value: token.access_token.secret().to_string() }
                } else {
                    self.fetch_token().await?
                }
            }
        };

        Ok(access_token)
    }

    pub async fn check_login(&self) -> Result<bool, AuthError> {
        let token = self.get_token().await?;
        Ok(!token.value.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use oauth2::{ClientId, ClientSecret};
    use pem::Pem;
    use rstest::{fixture, rstest};
    use url::Url;

    use opendut_util::project;

    use crate::carl::auth::auth_config::OidcIdentityProviderConfig;
    use crate::carl::auth::manager::{AuthenticationManager, AuthenticationManagerRef};
    use crate::carl::auth::reqwest_client::OidcReqwestClient;
    use crate::carl::auth::reqwest_client::PemFromConfig;

    #[fixture]
    async fn authentication_manager() -> AuthenticationManagerRef {
        let idp_config: OidcIdentityProviderConfig = OidcIdentityProviderConfig {
            client_id: ClientId::new("opendut-edgar-client".to_string()),
            client_secret: ClientSecret::new("c7d6ace0-b90f-471a-bb62-a4ecac4150f8".to_string()),
            issuer_url: Url::parse("http://localhost:8081/realms/opendut/").unwrap(),
            scopes: vec![],
        };
        let ca_path = project::make_path_absolute("resources/development/tls/insecure-development-ca.pem")
            .expect("Could not resolve dev CA").into_os_string().into_string().unwrap();
        let result = <Pem as PemFromConfig>::from_file_path(&ca_path).await;
        let pem: Pem = result.expect("Could not load dev CA");
        let reqwest_client = OidcReqwestClient::from_pem(pem)
            .map_err(|cause| anyhow!("Failed to create reqwest client. Error: {}", cause)).unwrap();

        AuthenticationManager::from_oidc_config(idp_config, reqwest_client).await.unwrap()
    }

    #[rstest]
    #[tokio::test]
    #[ignore]
    async fn test_auth_manager_get_token(#[future] authentication_manager: AuthenticationManagerRef) {
        /*
         * This test is ignored because it requires a running keycloak server from the test environment.
         * To run this test, execute the following command: cargo test -- --include-ignored
         */
        let token = authentication_manager.await.get_token().await.unwrap();
        assert!(token.value.len() > 100);
    }
}
