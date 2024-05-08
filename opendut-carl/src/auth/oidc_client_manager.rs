use oauth2::{AccessToken, AuthUrl, ClientId as OAuthClientId, ClientSecret as OAuthClientSecret, RedirectUrl, TokenResponse, TokenUrl};
use oauth2::basic::{BasicClient};
use openidconnect::{ClientName, ClientUrl, RegistrationUrl};
use openidconnect::core::{CoreClientRegistrationRequest, CoreGrantType};
use openidconnect::registration::EmptyAdditionalClientMetadata;
use serde::{Deserialize, Serialize};
use tracing::debug;
use url::Url;
use opendut_carl_api::carl::auth::error::parse_oauth_request_error;

use opendut_carl_api::carl::auth::reqwest_client::{OidcReqwestClient};
use opendut_types::util::net::{ClientCredentials, ClientId, ClientSecret};

use crate::auth::idp_config::CarlIdentityProviderConfig;
use crate::resources::Id;
use crate::settings::CarlUrl;

pub const DEVICE_REDIRECT_URL: &str = "http://localhost:12345/device";

#[derive(Debug, Clone)]
pub struct OpenIdConnectClientManager {
    client: BasicClient,
    reqwest_client: OidcReqwestClient,
    registration_url: RegistrationUrl,
    device_redirect_url: RedirectUrl,
    pub issuer_url: Url,
    pub issuer_remote_url: Url,
    peer_credentials: Option<CommonPeerCredentials>,
    carl_url: CarlUrl,
}

#[derive(Debug)]
pub struct OAuthClientCredentials {
    pub client_id: OAuthClientId,
    pub client_secret: OAuthClientSecret,
}

impl From<OAuthClientCredentials> for ClientCredentials {
    fn from(value: OAuthClientCredentials) -> Self {
        Self {
            client_id: ClientId(value.client_id.to_string()),
            client_secret: ClientSecret(value.client_secret.secret().to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CarlScopes(pub String);


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommonPeerCredentials {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthenticationClientManagerError {
    #[error("Invalid configuration:\n  {error}")]
    InvalidConfiguration {
        error: String,
    },
    #[error("Failed request:\n {error}")]
    RequestError {
        error: String,
        inner: Box<dyn std::error::Error + Send + Sync>,  // RequestTokenError<OidcClientError<reqwest::Error>, BasicErrorResponse>
    },
    #[error("Failed to register new client:\n  {error}")]
    Registration {
        error: String,
    },
}

impl OpenIdConnectClientManager {
    /// issuer_url for keycloak includes realm name: http://localhost:8081/realms/opendut
    pub(crate) fn new(config: CarlIdentityProviderConfig) -> Result<Self, AuthenticationClientManagerError> {
        // TODO: reuse AuthenticationManager here

        if config.issuer_url.as_str().ends_with('/') {
            // keycloak auth url: http://localhost:8081/realms/opendut/protocol/openid-connect/auth
            let auth_url = AuthUrl::from_url(
                config.issuer_url.join("protocol/openid-connect/auth")
                    .map_err(|error| AuthenticationClientManagerError::InvalidConfiguration { error: format!("Invalid auth endpoint url: {}", error) })?
            );
            let token_url = TokenUrl::from_url(
                config.issuer_url.join("protocol/openid-connect/token")
                    .map_err(|error| AuthenticationClientManagerError::InvalidConfiguration { error: format!("Invalid token endpoint url: {}", error) })?
            );
            let registration_url = RegistrationUrl::from_url(
                config.issuer_url.join("clients-registrations/openid-connect")
                    .map_err(|error| AuthenticationClientManagerError::InvalidConfiguration { error: format!("Invalid registration endpoint URL: {}", error) })?
            );

            let device_redirect_url = RedirectUrl::new(DEVICE_REDIRECT_URL.to_string()).expect("Could not parse device redirect url");

            let client =
                BasicClient::new(
                    config.client_id,
                    Some(config.client_secret),
                    auth_url,
                    Some(token_url),
                );

            let reqwest_client = OidcReqwestClient::from_pem(config.issuer_ca)
                .map_err(|error| AuthenticationClientManagerError::InvalidConfiguration { error: format!("Failed to load certificate authority. {}", error) })?;
            let manager = Ok(OpenIdConnectClientManager {
                client,
                reqwest_client,
                registration_url,
                device_redirect_url,
                issuer_url: config.issuer_url.clone(),
                issuer_remote_url: config.issuer_remote_url.clone(),
                peer_credentials: config.peer_credentials,
                carl_url: config.carl_url,
            });
            debug!("Created OpenIdConnectClientManager: {:?}", manager);
            manager
        } else {
            Err(AuthenticationClientManagerError::InvalidConfiguration {
                error: "Issuer URL must end with a slash".to_string(),
            })
        }
    }

    async fn get_token(&self) -> Result<AccessToken, AuthenticationClientManagerError> {
        let response = self.client.exchange_client_credentials()
            .request_async(|request| { self.reqwest_client.async_http_client(request) })
            .await
            .map_err(|error| {
                let string = parse_oauth_request_error(&error);
                AuthenticationClientManagerError::RequestError { error: string, inner: error.into() }
            })?;
        Ok(response.access_token().clone())
    }

    pub async fn register_new_client(&self, resource_id: Id) -> Result<OAuthClientCredentials, AuthenticationClientManagerError> {
        match self.peer_credentials.clone() {
            Some(peer_credentials) => {
                Ok(OAuthClientCredentials {
                    client_id: OAuthClientId::new(peer_credentials.client_id.value()),
                    client_secret: OAuthClientSecret::new(peer_credentials.client_secret.value()),
                })
            }
            None => {
                let access_token = self.get_token().await?;
                let additional_metadata = EmptyAdditionalClientMetadata {};
                let redirect_uris = vec![self.device_redirect_url.clone()];
                let grant_types = vec![CoreGrantType::ClientCredentials];
                let request: CoreClientRegistrationRequest =
                    openidconnect::registration::ClientRegistrationRequest::new(redirect_uris, additional_metadata)
                        .set_grant_types(Some(grant_types));
                let registration_url = self.registration_url.clone();

                let client_name: ClientName = ClientName::new(resource_id.to_string());
                let resource_uri = self.carl_url.resource_url(resource_id)
                    .map_err(|error| AuthenticationClientManagerError::Registration {
                        error: format!("Failed to forge client url: {:?}", error),
                    })?;
                let client_home_uri = ClientUrl::new(String::from(resource_uri))
                    .map_err(|error| AuthenticationClientManagerError::Registration {
                        error: format!("Failed to forge client url: {:?}", error),
                    })?;
                let response = request
                    .set_initial_access_token(Some(access_token))
                    .set_client_name(Some(
                        vec![(None, client_name)]
                            .into_iter()
                            .collect(),
                    ))
                    .set_client_uri(Some(vec![(None, client_home_uri)]
                        .into_iter()
                        .collect()))
                    .register_async(&registration_url, move |request| {
                        self.reqwest_client.async_http_client(request)
                    }).await;
                match response {
                    Ok(response) => {
                        let client_id = response.client_id();
                        let client_secret = response.client_secret().expect("Confidential client required!");

                        Ok(OAuthClientCredentials {
                            client_id: client_id.clone(),
                            client_secret: client_secret.clone(),
                        })
                    }
                    Err(error) => {
                        Err(AuthenticationClientManagerError::Registration {
                            error: format!("{:?}", error),
                        })
                    }
                }
            }
        }
    }
}


#[cfg(test)]
pub mod tests {
    use googletest::assert_that;
    use googletest::matchers::eq;
    use http::{HeaderMap, HeaderValue};
    use oauth2::HttpRequest;
    use pem::Pem;
    use rstest::{fixture, rstest};
    use url::Url;

    use opendut_carl_api::carl::auth::reqwest_client::PemFromConfig;

    use super::*;

    async fn delete_client(manager: OpenIdConnectClientManager, client_id: &OAuthClientId, issuer_ca: Pem) -> Result<(), AuthenticationClientManagerError> {
        let access_token = manager.get_token().await?;
        let delete_client_url = manager.issuer_url.join("/admin/realms/opendut/clients/").unwrap().join(&format!("{}", client_id.to_string()))
            .map_err(|error| AuthenticationClientManagerError::InvalidConfiguration { error: format!("Invalid client URL: {}", error) })?;

        let mut headers = HeaderMap::new();
        let bearer_header = format!("Bearer {}", access_token.secret().as_str());
        let access_token_value = HeaderValue::from_str(&bearer_header)
            .map_err(|error| AuthenticationClientManagerError::InvalidConfiguration { error: error.to_string() })?;
        headers.insert(http::header::AUTHORIZATION, access_token_value);

        let request = HttpRequest {
            method: http::Method::DELETE,
            url: delete_client_url,
            headers,
            body: vec![],
        };

        let reqwest_client = OidcReqwestClient::from_pem(issuer_ca)
            .map_err(|error| AuthenticationClientManagerError::InvalidConfiguration { error: format!("Failed to load certificate authority. {}", error) })?;

        let response = reqwest_client.async_http_client(request)
            .await
            .map_err(|error| AuthenticationClientManagerError::Registration { error: error.to_string() })?;
        assert_eq!(response.status_code, 204, "Failed to delete client with id '{:?}': {:?}", client_id, response.body);

        Ok(())
    }

    #[fixture]
    pub fn issuer_certificate_authority() -> Pem {
        futures::executor::block_on(Pem::from_file_path("resources/development/tls/insecure-development-ca.pem"))
            .expect("Failed to resolve development ca in resources directory.")
    }

    #[fixture]
    pub fn oidc_client_manager(issuer_certificate_authority: Pem) -> OpenIdConnectClientManager {
        /*
         * Issuer URL for keycloak needs to align with FRONTEND_URL in Keycloak realm setting.
         * Localhost address is always fine, though.
         */
        let client_id = "opendut-carl-client".to_string();
        let client_secret = "6754d533-9442-4ee6-952a-97e332eca38e".to_string();
        //let issuer_url = "http://192.168.56.10:8081/realms/opendut/".to_string();  // This is the URL for the keycloak server in the test environment (valid in host system and opendut-vm)
        let issuer_url = "https://keycloak/realms/opendut/".to_string();  // This is the URL for the keycloak server in the test environment
        let issuer_remote_url = "https://keycloak/realms/opendut/".to_string();  // works inside OpenDuT-VM

        let carl_idp_config = CarlIdentityProviderConfig {
            client_id: OAuthClientId::new(client_id),
            client_secret: OAuthClientSecret::new(client_secret),
            issuer_url: Url::parse(&issuer_url).unwrap(),
            issuer_remote_url: Url::parse(&issuer_remote_url).unwrap(),
            issuer_ca: issuer_certificate_authority,
            scopes: vec![],
            peer_credentials: None,
            carl_url: CarlUrl::new(Url::parse("https://opendut-carl").unwrap()),
        };
        OpenIdConnectClientManager::new(carl_idp_config).unwrap()
    }

    #[rstest]
    #[tokio::test]
    #[ignore]
    async fn test_register_new_oidc_client(oidc_client_manager: OpenIdConnectClientManager, issuer_certificate_authority: Pem) {
        /*
         * This test is ignored because it requires a running keycloak server from the test environment.
         * To run this test, execute the following command: cargo test -- --include-ignored
         */
        println!("{:?}", oidc_client_manager);
        let resource_id = Id::random();
        let credentials = oidc_client_manager.register_new_client(resource_id).await.unwrap();
        println!("New client id: {}, secret: {}", credentials.client_id.to_string(), credentials.client_secret.secret().to_string());
        delete_client(oidc_client_manager, &credentials.client_id, issuer_certificate_authority).await.unwrap();
        assert_that!(credentials.client_id.to_string().len().gt(&10), eq(true));
    }
}
