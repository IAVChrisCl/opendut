use oauth2::basic::BasicErrorResponse;
use oauth2::RequestTokenError;
use reqwest::Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OidcClientError<T>
    where
        T: std::error::Error + 'static,
{
    #[error("Foo Error, request failed: {0}")]
    Foo(#[source] T),
    #[error("AuthReqwest Error, request failed: '{message}' status: '{status}' inner: '{inner}'")]
    AuthReqwest {
        message: String,
        status: String,
        inner: Error,
    },
    #[error("Failed to load custom certificate authority: {}", _0)]
    LoadCustomCA(String),
    #[error("Other error: {}", _0)]
    Other(String),
}

pub fn parse_oauth_request_error(error: &RequestTokenError<OidcClientError<Error>, BasicErrorResponse>) -> String {
    match error {
        RequestTokenError::ServerResponse(ref server_error) => {
            server_error.error().to_string()
        }
        RequestTokenError::Request(ref request_error) => {
            request_error.to_string()
        }
        RequestTokenError::Parse(ref error_token, ref _error_response) => {
            error_token.to_string()
        }
        RequestTokenError::Other(ref other) => {
            other.to_string()
        }
    }
}
