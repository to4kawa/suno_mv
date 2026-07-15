use super::protocol::BridgePayload;
use super::session::BridgeSession;
use super::transport::{BridgeTransport, BridgeTransportError, TransportConfig};

#[derive(Debug, Clone)]
pub struct HttpBridgeTransport {
    pub session: BridgeSession,
    pub config: TransportConfig,
}

impl HttpBridgeTransport {
    pub fn new(session: BridgeSession, config: TransportConfig) -> Self {
        Self { session, config }
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
