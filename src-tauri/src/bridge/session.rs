#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeSessionConfig {
    pub host: String,
    pub timeout_seconds: u64,
    pub max_body_mb: usize,
    pub max_timing_records: usize,
}

impl Default for BridgeSessionConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            timeout_seconds: 60,
            max_body_mb: 2,
            max_timing_records: 5000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeSession {
    pub request_id: String,
    pub bearer_token: String,
    pub config: BridgeSessionConfig,
}

impl BridgeSession {
    pub fn new(
        request_id: impl Into<String>,
        bearer_token: impl Into<String>,
        config: BridgeSessionConfig,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            bearer_token: bearer_token.into(),
            config,
        }
    }
}
