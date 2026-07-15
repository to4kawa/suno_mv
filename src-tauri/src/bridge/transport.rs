use super::protocol::{BridgePayload, BridgeProtocolError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportConfig {
    pub host: String,
    pub timeout_seconds: u64,
    pub max_body_mb: usize,
    pub max_timing_records: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            timeout_seconds: 60,
            max_body_mb: 2,
            max_timing_records: 5000,
        }
    }
}

pub trait BridgeTransport {
    fn receive_once(&self, expected_request_id: &str) -> Result<BridgePayload, BridgeTransportError>;
}

#[derive(Debug)]
pub enum BridgeTransportError {
    NotImplemented,
    Protocol(BridgeProtocolError),
}
