#[path = "../src/bridge/mod.rs"]
mod bridge;

use bridge::protocol::{
    AlignedLyricTiming, BridgePayload, BridgeProtocolError, PAYLOAD_TYPE_ALIGNED_LYRICS,
    PROTOCOL_VERSION, PROVIDER_SUNO,
};

fn valid_payload() -> BridgePayload {
    BridgePayload {
        protocol_version: PROTOCOL_VERSION,
        request_id: "request-1".to_string(),
        provider: PROVIDER_SUNO.to_string(),
        payload_type: PAYLOAD_TYPE_ALIGNED_LYRICS.to_string(),
        song_id: "song-1".to_string(),
        timings: vec![AlignedLyricTiming {
            text: "line one".to_string(),
            start_seconds: 0.0,
            end_seconds: 2.5,
        }],
    }
}

#[test]
fn validates_protocol_payload_shape() {
    assert_eq!(valid_payload().validate("request-1"), Ok(()));
}

#[test]
fn rejects_empty_song_id() {
    let payload = BridgePayload {
        song_id: " ".to_string(),
        ..valid_payload()
    };

    assert_eq!(
        payload.validate("request-1"),
        Err(BridgeProtocolError::MissingField("song_id"))
    );
}

#[test]
fn rejects_empty_timings() {
    let payload = BridgePayload {
        timings: Vec::new(),
        ..valid_payload()
    };

    assert_eq!(
        payload.validate("request-1"),
        Err(BridgeProtocolError::EmptyTimings)
    );
}

#[test]
fn rejects_negative_start_time() {
    let payload = BridgePayload {
        timings: vec![AlignedLyricTiming {
            text: "line one".to_string(),
            start_seconds: -0.1,
            end_seconds: 1.0,
        }],
        ..valid_payload()
    };

    assert_eq!(
        payload.validate("request-1"),
        Err(BridgeProtocolError::InvalidTiming {
            index: 0,
            reason: "start_seconds must be non-negative",
        })
    );
}
