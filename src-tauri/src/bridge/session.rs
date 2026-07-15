use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::error::Error;
use std::fmt;

const REQUEST_ID_BYTES: usize = 16;
const BEARER_TOKEN_BYTES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeSession {
    pub request_id: String,
    pub bearer_token: String,
}

impl BridgeSession {
    pub fn new(request_id: impl Into<String>, bearer_token: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            bearer_token: bearer_token.into(),
        }
    }

    pub fn generate() -> Result<Self, BridgeSessionError> {
        Ok(Self {
            request_id: generate_secret(REQUEST_ID_BYTES)?,
            bearer_token: generate_secret(BEARER_TOKEN_BYTES)?,
        })
    }
}

fn generate_secret(byte_len: usize) -> Result<String, BridgeSessionError> {
    let mut bytes = vec![0; byte_len];
    getrandom::getrandom(&mut bytes).map_err(|err| BridgeSessionError::Random(err.to_string()))?;
    Ok(encode_secret(&bytes))
}

fn encode_secret(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeSessionError {
    Random(String),
}

impl fmt::Display for BridgeSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BridgeSessionError::Random(err) => {
                write!(f, "failed to generate bridge session secret: {err}")
            }
        }
    }
}

impl Error for BridgeSessionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_non_empty_url_safe_session_values() {
        let session = BridgeSession::generate().expect("session");

        assert_eq!(session.request_id.len(), 22);
        assert_eq!(session.bearer_token.len(), 43);
        assert!(is_url_safe_no_pad(&session.request_id));
        assert!(is_url_safe_no_pad(&session.bearer_token));
    }

    #[test]
    fn generated_sessions_are_not_reused() {
        let first = BridgeSession::generate().expect("first session");
        let second = BridgeSession::generate().expect("second session");

        assert_ne!(first.request_id, second.request_id);
        assert_ne!(first.bearer_token, second.bearer_token);
    }

    #[test]
    fn encodes_without_padding() {
        assert_eq!(encode_secret(&[0, 1, 2, 3]), "AAECAw");
    }

    fn is_url_safe_no_pad(value: &str) -> bool {
        value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    }
}
