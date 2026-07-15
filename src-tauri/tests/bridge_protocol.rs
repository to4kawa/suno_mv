#[path = "../src/bridge/mod.rs"]
mod bridge;

use bridge::protocol::{
    AlignedLyricTiming, BridgePayload, BridgeProtocolError, PayloadType, PROTOCOL_VERSION,
    PROVIDER_SUNO,
};

fn valid_payload() -> BridgePayload {
    BridgePayload {
        protocol_version: PROTOCOL_VERSION,
        request_id: "request-1".to_string(),
        provider: PROVIDER_SUNO.to_string(),
        payload_type: PayloadType::AlignedLyrics,
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
fn deserializes_compact_protocol_fields() {
    let payload: BridgePayload = serde_json::from_str(
        r#"{
            "protocol_version": 1,
            "request_id": "request-1",
            "provider": "suno",
            "type": "aligned_lyrics",
            "song_id": "song-1",
            "timings": [
                {
                    "text": "line one",
                    "start_s": 0.0,
                    "end_s": 2.5
                }
            ]
        }"#,
    )
    .expect("valid compact bridge payload");

    assert_eq!(payload.payload_type, PayloadType::AlignedLyrics);
    assert_eq!(payload.timings[0].start_seconds, 0.0);
    assert_eq!(payload.timings[0].end_seconds, 2.5);
}

#[test]
fn serializes_compact_protocol_fields() {
    let value = serde_json::to_value(valid_payload()).expect("bridge payload json");

    assert_eq!(value["type"], "aligned_lyrics");
    assert_eq!(value["timings"][0]["start_s"], 0.0);
    assert_eq!(value["timings"][0]["end_s"], 2.5);
    assert!(value["timings"][0].get("start_seconds").is_none());
    assert!(value["timings"][0].get("end_seconds").is_none());
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
