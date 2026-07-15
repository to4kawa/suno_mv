use super::session::BridgeSession;
use super::transport::{BridgeTransport, BridgeTransportError};
use super::protocol::BridgePayload;

#[derive(Debug, Clone)]
pub struct HttpBridgeTransport {
    pub session: BridgeSession,
}

impl HttpBridgeTransport {
    pub fn new(session: BridgeSession) -> Self {
        Self { session }
    }
}

impl BridgeTransport for HttpBridgeTransport {
    fn receive_once(
        &self,
        _expected_request_id: &str,
    ) -> Result<BridgePayload, BridgeTransportError> {
        Err(BridgeTransportError::NotImplemented)
    }
}
