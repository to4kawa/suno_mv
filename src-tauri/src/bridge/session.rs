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
}
