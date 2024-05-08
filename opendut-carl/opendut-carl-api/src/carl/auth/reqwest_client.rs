use std::fs::File;
use std::io::Read;

use anyhow::anyhow;
use config::Config;
use oauth2::{HttpRequest, HttpResponse};
use pem::Pem;
use reqwest::Certificate;
use opendut_util::project;
use crate::carl::auth::error::OidcClientError;

#[derive(Debug, Clone)]
pub struct OidcReqwestClient {
    pub(crate) client: reqwest::Client,
}

const CONFIG_KEY_GENERIC_CA: &str = "network.tls.ca";
const CONFIG_KEY_OIDC_CA: &str = "network.oidc.client.ca";

impl OidcReqwestClient {
    pub async fn from_config(config: &Config) -> anyhow::Result<Self> {
        match Pem::from_config_path(CONFIG_KEY_OIDC_CA, config).await {
            Ok(ca_certificate) => {
                let client = OidcReqwestClient::build_client(ca_certificate)?;
                Ok(Self { client })
            }
            Err(_error) => {
                // could not find specific OIDC CA, try generic CA
                match Pem::from_config_path(CONFIG_KEY_GENERIC_CA, config).await {
                    Ok(ca_certificate) => {
                        Ok(Self { client: OidcReqwestClient::build_client(ca_certificate)? })
                    }
                    Err(error) => {
                        Err(anyhow!("Could not find any CA certificate in config. Error: {}", error))
                    }
                }
            }
        }
    }

    fn build_client(ca_certificate: Pem) -> anyhow::Result<reqwest::Client> {
        let reqwest_certificate = Certificate::from_pem(ca_certificate.to_string().as_bytes().iter().as_slice())
            .map_err(|cause| OidcClientError::<reqwest::Error>::LoadCustomCA(cause.to_string()))?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .add_root_certificate(reqwest_certificate)
            .build()
            .map_err(|cause| OidcClientError::<reqwest::Error>::LoadCustomCA(cause.to_string()))?;
        Ok(client)
    }

    pub fn from_pem(ca_certificate: Pem) -> anyhow::Result<Self> {
        Ok(Self { client: OidcReqwestClient::build_client(ca_certificate)? })
    }

    pub fn client(&self) -> reqwest::Client {
        self.client.clone()
    }

    pub async fn async_http_client(
        &self,
        request: HttpRequest,
    ) -> Result<HttpResponse, OidcClientError<reqwest::Error>> {
        let client = self.client.clone();
        let mut request_builder = client
            .request(request.method, request.url.as_str())
            .body(request.body);
        for (name, value) in &request.headers {
            request_builder = request_builder.header(name.as_str(), value.as_bytes());
        }
        let request = request_builder.build()
            .map_err(|cause| {
                OidcClientError::AuthReqwest { message: cause.to_string(), status: cause.status().unwrap_or_default().to_string(), inner: cause }
            })?;
        let response = client.execute(request).await
            .map_err(|cause: reqwest::Error| {
                // TODO: differentiate connection error (repeatable?) from other errors
                println!("XXXX: Request failed error: {}", cause);
                println!("XXXX: connect: {}", cause.is_connect());
                println!("XXXX: request: {}", cause.is_request());
                println!("XXXX: timeout: {}", cause.is_timeout());
                println!("XXXX: body: {}", cause.is_body());
                println!("XXXX: code: {}", cause.is_status());
                println!("XXXX: status_code: {:?}", cause.status());

                OidcClientError::AuthReqwest { message: cause.to_string(), status: cause.status().unwrap_or_default().to_string(), inner: cause }
            })?;
        let status_code = response.status();
        let headers = response.headers().to_owned();
        let data = response.bytes().await
            .map_err(|cause| {
                OidcClientError::AuthReqwest { message: cause.to_string(), status: cause.status().unwrap_or_default().to_string(), inner: cause }
            })?;
        Ok(HttpResponse {
            status_code,
            headers,
            body: data.to_vec(),
        })
    }
}


pub trait PemFromConfig {
    fn from_config_path(config_key: &str, config: &Config) -> impl std::future::Future<Output=anyhow::Result<Pem>> + Send;
    fn from_file_path(relative_file_path: &str) -> impl std::future::Future<Output=anyhow::Result<Pem>> + Send;
}

impl PemFromConfig for Pem {
    async fn from_config_path(config_key: &str, config: &Config) -> anyhow::Result<Pem> {
        let ca_file_path = project::make_path_absolute(config.get_string(config_key)?)?;
        read_pem_from_file_path(ca_file_path.to_str().ok_or(anyhow!("foo"))?)
    }

    async fn from_file_path(relative_file_path: &str) -> anyhow::Result<Pem> {
        let ca_file_path = project::make_path_absolute(relative_file_path)?;
        read_pem_from_file_path(ca_file_path.to_str().ok_or(anyhow!("foo"))?)
    }
}

fn read_pem_from_file_path(ca_file_path: &str) -> anyhow::Result<Pem> {
    let mut buffer = Vec::new();
    let mut ca_file = File::open(ca_file_path)?;
    ca_file.read_to_end(&mut buffer)
        .map_err(|cause| OidcClientError::<reqwest::Error>::LoadCustomCA(cause.to_string()))?;
    let ca_certificate = Pem::try_from(buffer.as_slice())
        .map_err(|cause| anyhow!("Could not load CA certificate from filesystem path given: {}. {:?}", ca_file_path, cause))?;
    Ok(ca_certificate)
}
