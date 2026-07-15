pub mod http;
pub mod protocol;
pub mod session;
pub mod transport;

use session::{BridgeSession, BridgeSessionError};
use transport::TransportConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserBridge {
    pub session: BridgeSession,
    pub transport_config: TransportConfig,
}

pub fn create_browser_bridge() -> Result<BrowserBridge, BridgeSessionError> {
    Ok(BrowserBridge {
        session: BridgeSession::generate()?,
        transport_config: TransportConfig::default(),
    })
}
