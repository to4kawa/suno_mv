use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

pub const PROTOCOL_VERSION: u16 = 1;
pub const PROVIDER_SUNO: &str = "suno";
pub const MAX_TIMING_RECORDS: usize = 5000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BridgePayload {
    pub protocol_version: u16,
    pub request_id: String,
    pub provider: String,
    #[serde(rename = "type")]
    pub payload_type: PayloadType,
    pub song_id: String,
    pub timings: Vec<AlignedLyricTiming>,
}

impl BridgePayload {
    pub fn validate(&self, expected_request_id: &str) -> Result<(), BridgeProtocolError> {
        self.validate_with_limits(expected_request_id, MAX_TIMING_RECORDS)
    }

    pub fn validate_with_limits(
        &self,
        expected_request_id: &str,
        max_timing_records: usize,
    ) -> Result<(), BridgeProtocolError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(BridgeProtocolError::UnsupportedProtocolVersion {
                actual: self.protocol_version,
                expected: PROTOCOL_VERSION,
            });
        }

        if self.request_id.trim().is_empty() {
            return Err(BridgeProtocolError::MissingField("request_id"));
        }

        if self.request_id != expected_request_id {
            return Err(BridgeProtocolError::RequestIdMismatch);
        }

        if self.provider != PROVIDER_SUNO {
            return Err(BridgeProtocolError::UnsupportedProvider(self.provider.clone()));
        }

        if self.song_id.trim().is_empty() {
            return Err(BridgeProtocolError::MissingField("song_id"));
        }

        if self.timings.is_empty() {
            return Err(BridgeProtocolError::EmptyTimings);
        }

        if self.timings.len() > max_timing_records {
            return Err(BridgeProtocolError::TooManyTimingRecords {
                actual: self.timings.len(),
                max: max_timing_records,
            });
        }

        for (index, timing) in self.timings.iter().enumerate() {
            timing.validate(index)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PayloadType {
    AlignedLyrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlignedLyricTiming {
    pub text: String,
    #[serde(rename = "start_s")]
    pub start_seconds: f64,
    #[serde(rename = "end_s")]
    pub end_seconds: f64,
}

impl AlignedLyricTiming {
    fn validate(&self, index: usize) -> Result<(), BridgeProtocolError> {
        if self.text.trim().is_empty() {
            return Err(BridgeProtocolError::InvalidTiming {
                index,
                reason: "text is required",
            });
        }

        if !self.start_seconds.is_finite() || !self.end_seconds.is_finite() {
            return Err(BridgeProtocolError::InvalidTiming {
                index,
                reason: "timestamps must be finite",
            });
        }

        if self.start_seconds < 0.0 {
            return Err(BridgeProtocolError::InvalidTiming {
                index,
                reason: "start_seconds must be non-negative",
            });
        }

        if self.end_seconds < self.start_seconds {
            return Err(BridgeProtocolError::InvalidTiming {
                index,
                reason: "end_seconds must be greater than or equal to start_seconds",
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeProtocolError {
    UnsupportedProtocolVersion { actual: u16, expected: u16 },
    MissingField(&'static str),
    RequestIdMismatch,
    UnsupportedProvider(String),
    EmptyTimings,
    TooManyTimingRecords { actual: usize, max: usize },
    InvalidTiming { index: usize, reason: &'static str },
}

impl fmt::Display for BridgeProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BridgeProtocolError::UnsupportedProtocolVersion { actual, expected } => {
                write!(
                    f,
                    "unsupported protocol_version {actual}; expected {expected}"
                )
            }
            BridgeProtocolError::MissingField(field) => write!(f, "missing required field {field}"),
            BridgeProtocolError::RequestIdMismatch => write!(f, "request_id does not match session"),
            BridgeProtocolError::UnsupportedProvider(provider) => {
                write!(f, "unsupported provider {provider}")
            }
            BridgeProtocolError::EmptyTimings => write!(f, "timings must not be empty"),
            BridgeProtocolError::TooManyTimingRecords { actual, max } => {
                write!(f, "too many timing records: {actual}; max {max}")
            }
            BridgeProtocolError::InvalidTiming { index, reason } => {
                write!(f, "invalid timing at index {index}: {reason}")
            }
        }
    }
}

impl Error for BridgeProtocolError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_payload() -> BridgePayload {
        BridgePayload {
            protocol_version: PROTOCOL_VERSION,
            request_id: "request-1".to_string(),
            provider: PROVIDER_SUNO.to_string(),
            payload_type: PayloadType::AlignedLyrics,
            song_id: "song-1".to_string(),
            timings: vec![AlignedLyricTiming {
                text: "hello".to_string(),
                start_seconds: 0.0,
                end_seconds: 1.25,
            }],
        }
    }

    #[test]
    fn accepts_valid_aligned_lyrics_payload() {
        assert_eq!(valid_payload().validate("request-1"), Ok(()));
    }

    #[test]
    fn rejects_request_id_mismatch() {
        assert_eq!(
            valid_payload().validate("different-request"),
            Err(BridgeProtocolError::RequestIdMismatch)
        );
    }

    #[test]
    fn rejects_unsupported_protocol_version() {
        let payload = BridgePayload {
            protocol_version: 2,
            ..valid_payload()
        };

        assert_eq!(
            payload.validate("request-1"),
            Err(BridgeProtocolError::UnsupportedProtocolVersion {
                actual: 2,
                expected: PROTOCOL_VERSION,
            })
        );
    }

    #[test]
    fn rejects_unsupported_provider() {
        let payload = BridgePayload {
            provider: "other".to_string(),
            ..valid_payload()
        };

        assert_eq!(
            payload.validate("request-1"),
            Err(BridgeProtocolError::UnsupportedProvider("other".to_string()))
        );
    }

    #[test]
    fn rejects_empty_timing_text() {
        let payload = BridgePayload {
            timings: vec![AlignedLyricTiming {
                text: " ".to_string(),
                start_seconds: 0.0,
                end_seconds: 1.0,
            }],
            ..valid_payload()
        };

        assert_eq!(
            payload.validate("request-1"),
            Err(BridgeProtocolError::InvalidTiming {
                index: 0,
                reason: "text is required",
            })
        );
    }

    #[test]
    fn rejects_invalid_timing_range() {
        let payload = BridgePayload {
            timings: vec![AlignedLyricTiming {
                text: "hello".to_string(),
                start_seconds: 2.0,
                end_seconds: 1.0,
            }],
            ..valid_payload()
        };

        assert_eq!(
            payload.validate("request-1"),
            Err(BridgeProtocolError::InvalidTiming {
                index: 0,
                reason: "end_seconds must be greater than or equal to start_seconds",
            })
        );
    }
}
