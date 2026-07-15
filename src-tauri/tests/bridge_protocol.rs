#[path = "../src/bridge/mod.rs"]
mod bridge;

use bridge::http::{parse_bridge_request, HttpRequestParseError, HttpRequestParts};
use bridge::protocol::{
    AlignedLyricTiming, BridgePayload, BridgeProtocolError, PayloadType, PROTOCOL_VERSION,
    PROVIDER_SUNO,
};
use bridge::session::BridgeSession;
use bridge::transport::TransportConfig;

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

fn bridge_session() -> BridgeSession {
    BridgeSession::new("request-1", "token-1")
}

fn transport_config() -> TransportConfig {
    TransportConfig {
        max_body_mb: 1,
        max_timing_records: 10,
        ..TransportConfig::default()
    }
}

fn valid_request() -> HttpRequestParts {
    HttpRequestParts::new(
        "POST",
        "/v1/result",
        Some("Bearer token-1".to_string()),
        serde_json::to_vec(&valid_payload()).expect("valid payload json"),
    )
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
fn parses_valid_http_request_parts() {
    let payload =
        parse_bridge_request(&valid_request(), &bridge_session(), &transport_config()).unwrap();

    assert_eq!(payload.song_id, "song-1");
    assert_eq!(payload.payload_type, PayloadType::AlignedLyrics);
}

#[test]
fn rejects_non_post_http_request_parts() {
    let request = HttpRequestParts {
        method: "GET".to_string(),
        ..valid_request()
    };

    assert_eq!(
        parse_bridge_request(&request, &bridge_session(), &transport_config()),
        Err(HttpRequestParseError::InvalidMethod)
    );
}

#[test]
fn rejects_wrong_http_path() {
    let request = HttpRequestParts {
        path: "/wrong".to_string(),
        ..valid_request()
    };

    assert_eq!(
        parse_bridge_request(&request, &bridge_session(), &transport_config()),
        Err(HttpRequestParseError::InvalidPath)
    );
}

#[test]
fn rejects_missing_bearer_auth() {
    let request = HttpRequestParts {
        authorization: None,
        ..valid_request()
    };

    assert_eq!(
        parse_bridge_request(&request, &bridge_session(), &transport_config()),
        Err(HttpRequestParseError::MissingAuthorization)
    );
}

#[test]
fn rejects_wrong_bearer_token() {
    let request = HttpRequestParts {
        authorization: Some("Bearer wrong".to_string()),
        ..valid_request()
    };

    assert_eq!(
        parse_bridge_request(&request, &bridge_session(), &transport_config()),
        Err(HttpRequestParseError::InvalidAuthorization)
    );
}

#[test]
fn rejects_body_over_configured_limit() {
    let config = TransportConfig {
        max_body_mb: 0,
        ..transport_config()
    };

    assert_eq!(
        parse_bridge_request(&valid_request(), &bridge_session(), &config),
        Err(HttpRequestParseError::BodyTooLarge {
            actual: valid_request().body.len(),
            max: 0,
        })
    );
}

#[test]
fn rejects_invalid_json_body() {
    let request = HttpRequestParts {
        body: b"{".to_vec(),
        ..valid_request()
    };

    assert!(matches!(
        parse_bridge_request(&request, &bridge_session(), &transport_config()),
        Err(HttpRequestParseError::InvalidJson(_))
    ));
}

#[test]
fn rejects_protocol_errors_from_http_request_parts() {
    let payload = BridgePayload {
        request_id: "wrong-request".to_string(),
        ..valid_payload()
    };
    let request = HttpRequestParts {
        body: serde_json::to_vec(&payload).expect("payload json"),
        ..valid_request()
    };

    assert_eq!(
        parse_bridge_request(&request, &bridge_session(), &transport_config()),
        Err(HttpRequestParseError::Protocol(
            BridgeProtocolError::RequestIdMismatch
        ))
    );
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
