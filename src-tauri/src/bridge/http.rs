use super::protocol::BridgePayload;
use super::session::BridgeSession;
use super::transport::{BridgeTransport, BridgeTransportError, TransportConfig};
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

const RESULT_ENDPOINT: &str = "/v1/result";
const BEARER_PREFIX: &str = "Bearer ";
const HEADER_END: &[u8] = b"\r\n\r\n";
const READ_CHUNK_BYTES: usize = 8192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestParts {
    pub method: String,
    pub path: String,
    pub authorization: Option<String>,
    pub body: Vec<u8>,
}

impl HttpRequestParts {
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        authorization: Option<String>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            authorization,
            body,
        }
    }
}

pub fn parse_bridge_request(
    request: &HttpRequestParts,
    session: &BridgeSession,
    config: &TransportConfig,
) -> Result<BridgePayload, HttpRequestParseError> {
    if request.method != "POST" {
        return Err(HttpRequestParseError::InvalidMethod);
    }

    if request.path != RESULT_ENDPOINT {
        return Err(HttpRequestParseError::InvalidPath);
    }

    validate_authorization(request.authorization.as_deref(), &session.bearer_token)?;

    let max_body_bytes = config.max_body_mb.saturating_mul(1024 * 1024);
    if request.body.len() > max_body_bytes {
        return Err(HttpRequestParseError::BodyTooLarge {
            actual: request.body.len(),
            max: max_body_bytes,
        });
    }

    let payload: BridgePayload = serde_json::from_slice(&request.body)
        .map_err(|err| HttpRequestParseError::InvalidJson(err.to_string()))?;
    payload
        .validate_with_limits(&session.request_id, config.max_timing_records)
        .map_err(HttpRequestParseError::Protocol)?;

    Ok(payload)
}

fn validate_authorization(
    authorization: Option<&str>,
    expected_token: &str,
) -> Result<(), HttpRequestParseError> {
    let Some(authorization) = authorization else {
        return Err(HttpRequestParseError::MissingAuthorization);
    };

    let Some(token) = authorization.strip_prefix(BEARER_PREFIX) else {
        return Err(HttpRequestParseError::InvalidAuthorization);
    };

    if token != expected_token {
        return Err(HttpRequestParseError::InvalidAuthorization);
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum HttpRequestParseError {
    InvalidMethod,
    InvalidPath,
    MissingAuthorization,
    InvalidAuthorization,
    BodyTooLarge { actual: usize, max: usize },
    InvalidJson(String),
    Protocol(super::protocol::BridgeProtocolError),
}

impl fmt::Display for HttpRequestParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpRequestParseError::InvalidMethod => write!(f, "expected POST request"),
            HttpRequestParseError::InvalidPath => write!(f, "expected /v1/result endpoint"),
            HttpRequestParseError::MissingAuthorization => {
                write!(f, "missing authorization header")
            }
            HttpRequestParseError::InvalidAuthorization => {
                write!(f, "invalid authorization header")
            }
            HttpRequestParseError::BodyTooLarge { actual, max } => {
                write!(f, "request body too large: {actual}; max {max}")
            }
            HttpRequestParseError::InvalidJson(err) => write!(f, "invalid JSON payload: {err}"),
            HttpRequestParseError::Protocol(err) => write!(f, "{err}"),
        }
    }
}

impl Error for HttpRequestParseError {}

pub struct HttpBridgeTransport {
    pub config: TransportConfig,
    listener: TcpListener,
    local_addr: SocketAddr,
}

impl HttpBridgeTransport {
    pub fn bind(config: TransportConfig) -> Result<Self, BridgeTransportError> {
        if config.host != "127.0.0.1" {
            return Err(BridgeTransportError::BindFailed(
                "HTTP bridge transport only binds to 127.0.0.1".to_string(),
            ));
        }

        let listener = TcpListener::bind((config.host.as_str(), 0))
            .map_err(|err| BridgeTransportError::BindFailed(err.to_string()))?;
        listener
            .set_nonblocking(true)
            .map_err(|err| BridgeTransportError::Io(err.to_string()))?;
        let local_addr = listener
            .local_addr()
            .map_err(|err| BridgeTransportError::Io(err.to_string()))?;

        Ok(Self {
            config,
            listener,
            local_addr,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn port(&self) -> u16 {
        self.local_addr.port()
    }
}

impl BridgeTransport for HttpBridgeTransport {
    fn receive_once(&self, session: &BridgeSession) -> Result<BridgePayload, BridgeTransportError> {
        let deadline = Instant::now() + Duration::from_secs(self.config.timeout_seconds);

        loop {
            match self.listener.accept() {
                Ok((mut stream, _peer_addr)) => {
                    let result = handle_connection(&mut stream, session, &self.config);
                    write_response(&mut stream, &result);
                    return result;
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(BridgeTransportError::Timeout);
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => return Err(BridgeTransportError::Io(err.to_string())),
            }
        }
    }
}

fn handle_connection(
    stream: &mut TcpStream,
    session: &BridgeSession,
    config: &TransportConfig,
) -> Result<BridgePayload, BridgeTransportError> {
    let timeout = Duration::from_secs(config.timeout_seconds);
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|err| BridgeTransportError::Io(err.to_string()))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|err| BridgeTransportError::Io(err.to_string()))?;

    let request = read_http_request(stream, config)?;
    parse_bridge_request(&request, session, config).map_err(|err| match err {
        HttpRequestParseError::Protocol(protocol_err) => {
            BridgeTransportError::Protocol(protocol_err)
        }
        other => BridgeTransportError::RequestRejected(other.to_string()),
    })
}

fn read_http_request(
    stream: &mut TcpStream,
    config: &TransportConfig,
) -> Result<HttpRequestParts, BridgeTransportError> {
    let max_body_bytes = config.max_body_mb.saturating_mul(1024 * 1024);
    let max_request_bytes = max_body_bytes.saturating_add(READ_CHUNK_BYTES);
    let mut buffer = Vec::new();
    let mut chunk = [0; READ_CHUNK_BYTES];

    loop {
        let read = stream
            .read(&mut chunk)
            .map_err(|err| BridgeTransportError::Io(err.to_string()))?;
        if read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > max_request_bytes {
            return Err(BridgeTransportError::RequestRejected(
                "request exceeds configured body limit".to_string(),
            ));
        }

        if let Some(header_end) = find_header_end(&buffer) {
            let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
            let content_length = content_length(&headers)?;
            if content_length > max_body_bytes {
                return Err(BridgeTransportError::RequestRejected(
                    HttpRequestParseError::BodyTooLarge {
                        actual: content_length,
                        max: max_body_bytes,
                    }
                    .to_string(),
                ));
            }

            let body_start = header_end + HEADER_END.len();
            while buffer.len() < body_start + content_length {
                let read = stream
                    .read(&mut chunk)
                    .map_err(|err| BridgeTransportError::Io(err.to_string()))?;
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
            }

            if buffer.len() < body_start + content_length {
                return Err(BridgeTransportError::RequestRejected(
                    "request body ended before content-length was satisfied".to_string(),
                ));
            }

            let body = buffer[body_start..body_start + content_length].to_vec();
            return request_parts_from_headers(&headers, body);
        }
    }

    Err(BridgeTransportError::RequestRejected(
        "request did not include complete HTTP headers".to_string(),
    ))
}

fn request_parts_from_headers(
    headers: &str,
    body: Vec<u8>,
) -> Result<HttpRequestParts, BridgeTransportError> {
    let mut lines = headers.lines();
    let request_line = lines.next().ok_or_else(|| {
        BridgeTransportError::RequestRejected("missing HTTP request line".to_string())
    })?;
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts
        .next()
        .ok_or_else(|| BridgeTransportError::RequestRejected("missing HTTP method".to_string()))?;
    let path = request_line_parts
        .next()
        .ok_or_else(|| BridgeTransportError::RequestRejected("missing HTTP path".to_string()))?;
    let authorization = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("authorization") {
            Some(value.trim().to_string())
        } else {
            None
        }
    });

    Ok(HttpRequestParts::new(method, path, authorization, body))
}

fn content_length(headers: &str) -> Result<usize, BridgeTransportError> {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                Some(value.trim())
            } else {
                None
            }
        })
        .unwrap_or("0")
        .parse::<usize>()
        .map_err(|_| {
            BridgeTransportError::RequestRejected("invalid content-length header".to_string())
        })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(HEADER_END.len())
        .position(|window| window == HEADER_END)
}

fn write_response(stream: &mut TcpStream, result: &Result<BridgePayload, BridgeTransportError>) {
    let (status, body) = match result {
        Ok(_) => ("200 OK", "ok"),
        Err(BridgeTransportError::Timeout) => ("408 Request Timeout", "timeout"),
        Err(BridgeTransportError::Protocol(_)) | Err(BridgeTransportError::RequestRejected(_)) => {
            ("400 Bad Request", "bad request")
        }
        Err(BridgeTransportError::BindFailed(_)) | Err(BridgeTransportError::Io(_)) => {
            ("500 Internal Server Error", "internal error")
        }
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::protocol::{
        AlignedLyricTiming, BridgePayload, PayloadType, PROTOCOL_VERSION, PROVIDER_SUNO,
    };
    use crate::bridge::transport::BridgeTransport;

    fn session() -> BridgeSession {
        BridgeSession::new("request-1", "token-1")
    }

    fn config() -> TransportConfig {
        TransportConfig {
            timeout_seconds: 1,
            max_body_mb: 1,
            max_timing_records: 10,
            ..TransportConfig::default()
        }
    }

    fn payload_json(request_id: &str) -> String {
        let payload = BridgePayload {
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.to_string(),
            provider: PROVIDER_SUNO.to_string(),
            payload_type: PayloadType::AlignedLyrics,
            song_id: "song-1".to_string(),
            timings: vec![AlignedLyricTiming {
                text: "line one".to_string(),
                start_seconds: 0.0,
                end_seconds: 1.0,
            }],
        };
        serde_json::to_string(&payload).expect("payload json")
    }

    fn post_request(port: u16, path: &str, token: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to transport");
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).expect("write request");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read response");
        response
    }

    #[test]
    fn binds_to_dynamic_localhost_port() {
        let transport = HttpBridgeTransport::bind(config()).expect("bind transport");

        assert_eq!(transport.local_addr().ip().to_string(), "127.0.0.1");
        assert_ne!(transport.port(), 0);
    }

    #[test]
    fn rejects_non_localhost_bind_config() {
        let config = TransportConfig {
            host: "0.0.0.0".to_string(),
            ..config()
        };

        assert!(matches!(
            HttpBridgeTransport::bind(config),
            Err(BridgeTransportError::BindFailed(_))
        ));
    }

    #[test]
    fn receives_exactly_one_valid_post() {
        let transport = HttpBridgeTransport::bind(config()).expect("bind transport");
        let port = transport.port();
        let session = session();
        let handle = thread::spawn(move || transport.receive_once(&session));

        let response = post_request(port, RESULT_ENDPOINT, "token-1", &payload_json("request-1"));
        let payload = handle
            .join()
            .expect("transport thread")
            .expect("valid payload");

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert_eq!(payload.song_id, "song-1");
    }

    #[test]
    fn shuts_down_after_rejected_request() {
        let transport = HttpBridgeTransport::bind(config()).expect("bind transport");
        let port = transport.port();
        let session = session();
        let handle = thread::spawn(move || transport.receive_once(&session));

        let response = post_request(
            port,
            RESULT_ENDPOINT,
            "wrong-token",
            &payload_json("request-1"),
        );
        let result = handle.join().expect("transport thread");

        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(matches!(
            result,
            Err(BridgeTransportError::RequestRejected(_))
        ));
    }

    #[test]
    fn returns_timeout_without_request() {
        let config = TransportConfig {
            timeout_seconds: 0,
            ..config()
        };
        let transport = HttpBridgeTransport::bind(config).expect("bind transport");

        assert_eq!(
            transport.receive_once(&session()),
            Err(BridgeTransportError::Timeout)
        );
    }
}
