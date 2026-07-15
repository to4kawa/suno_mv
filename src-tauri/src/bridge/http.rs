use super::protocol::BridgePayload;
use super::session::BridgeSession;
use super::transport::{BridgeTransport, BridgeTransportError, TransportConfig};
use std::error::Error;
use std::fmt;

const RESULT_ENDPOINT: &str = "/v1/result";
const BEARER_PREFIX: &str = "Bearer ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestParts {
    pub method: String,
    pub path: String,
    pub authorization: Option<String>,
    pub body: Vec<u8>,
}

impl HttpRequestParts {
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        authorization: Option<String>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            authorization,
            body,
        }
    }
}

pub fn parse_bridge_request(
    request: &HttpRequestParts,
    session: &BridgeSession,
    config: &TransportConfig,
) -> Result<BridgePayload, HttpRequestParseError> {
    if request.method != "POST" {
        return Err(HttpRequestParseError::InvalidMethod);
    }

    if request.path != RESULT_ENDPOINT {
        return Err(HttpRequestParseError::InvalidPath);
    }

    validate_authorization(request.authorization.as_deref(), &session.bearer_token)?;

    let max_body_bytes = config.max_body_mb.saturating_mul(1024 * 1024);
    if request.body.len() > max_body_bytes {
        return Err(HttpRequestParseError::BodyTooLarge {
            actual: request.body.len(),
            max: max_body_bytes,
        });
    }

    let payload: BridgePayload = serde_json::from_slice(&request.body)
        .map_err(|err| HttpRequestParseError::InvalidJson(err.to_string()))?;
    payload
        .validate_with_limits(&session.request_id, config.max_timing_records)
        .map_err(HttpRequestParseError::Protocol)?;

    Ok(payload)
}

fn validate_authorization(
    authorization: Option<&str>,
    expected_token: &str,
) -> Result<(), HttpRequestParseError> {
    let Some(authorization) = authorization else {
        return Err(HttpRequestParseError::MissingAuthorization);
    };

    let Some(token) = authorization.strip_prefix(BEARER_PREFIX) else {
        return Err(HttpRequestParseError::InvalidAuthorization);
    };

    if token != expected_token {
        return Err(HttpRequestParseError::InvalidAuthorization);
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum HttpRequestParseError {
    InvalidMethod,
    InvalidPath,
    MissingAuthorization,
    InvalidAuthorization,
    BodyTooLarge { actual: usize, max: usize },
    InvalidJson(String),
    Protocol(super::protocol::BridgeProtocolError),
}

impl fmt::Display for HttpRequestParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpRequestParseError::InvalidMethod => write!(f, "expected POST request"),
            HttpRequestParseError::InvalidPath => write!(f, "expected /v1/result endpoint"),
            HttpRequestParseError::MissingAuthorization => {
                write!(f, "missing authorization header")
            }
            HttpRequestParseError::InvalidAuthorization => {
                write!(f, "invalid authorization header")
            }
            HttpRequestParseError::BodyTooLarge { actual, max } => {
                write!(f, "request body too large: {actual}; max {max}")
            }
            HttpRequestParseError::InvalidJson(err) => write!(f, "invalid JSON payload: {err}"),
            HttpRequestParseError::Protocol(err) => write!(f, "{err}"),
        }
    }
}

impl Error for HttpRequestParseError {}

#[derive(Debug, Clone)]
pub struct HttpBridgeTransport {
    pub config: TransportConfig,
}

impl HttpBridgeTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self { config }
    }
}

impl BridgeTransport for HttpBridgeTransport {
    fn receive_once(&self, _session: &BridgeSession) -> Result<BridgePayload, BridgeTransportError> {
        Err(BridgeTransportError::NotImplemented)
    }
}
