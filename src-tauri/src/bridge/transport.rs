use super::protocol::{BridgePayload, BridgeProtocolError};

pub trait BridgeTransport {
    fn receive_once(&self, expected_request_id: &str) -> Result<BridgePayload, BridgeTransportError>;
}

#[derive(Debug)]
pub enum BridgeTransportError {
    NotImplemented,
    Protocol(BridgeProtocolError),
}
