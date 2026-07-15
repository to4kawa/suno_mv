# REVIEW-0002 Browser Bridge HTTP

## Goal

Review implementation plan before coding.

## Codex Summary


**1. Implementation Goal**

Implement the Browser Bridge proof in small stages: a one-shot localhost bridge that lets an authenticated browser userscript POST processed Suno data into the Rust backend, without transferring browser credentials. The immediate next work should be HTTP transport only, still disconnected from `lib.rs`, UI, and FFmpeg.

**2. Architecture**

The intended flow is:

Authenticated browser → Userscript → JSON → temporary `127.0.0.1:{dynamic_port}` bridge → Rust validation/processing → shutdown

Core rules from the docs:

- Browser owns authentication.
- Rust must not receive cookies, authorization headers from Suno, browser tokens, sessions, or Google credentials.
- The bridge is one-shot, localhost-only, dynamic-port, bearer-protected, and request-id scoped.
- The first proof payload is `aligned_lyrics`; SRT/FFmpeg integration is explicitly not the point yet.
- Existing MV generation must remain functional.
- New architecture should start isolated and only integrate after module/interface review.

**3. Rust Module Responsibilities**

Current/refined bridge skeleton responsibilities should be:

- `bridge/mod.rs`
  - Module boundary only while disconnected.
  - Later: expose a small command-facing API once integration is approved.

- `bridge/protocol.rs`
  - Pure protocol DTOs.
  - Serde mapping between compact JSON protocol and expressive Rust fields.
  - `PayloadType` enum.
  - Protocol validation: version, provider, type, request id, song id, timing shape, limits.
  - Unit tests for parsing/validation.

- `bridge/session.rs`
  - Bridge session identity/lifetime data only:
    - `request_id`
    - `bearer_token`
    - later `expires_at`
  - No host/body/timeout config.

- `bridge/transport.rs`
  - Transport-agnostic trait.
  - Shared `TransportConfig`.
  - Shared transport error type.
  - No HTTP assumptions beyond generic receive behavior.

- `bridge/http.rs`
  - HTTP-specific implementation only.
  - Later: bind `127.0.0.1:0`, accept `POST /v1/result`, check bearer auth, enforce max body, timeout, one-shot shutdown.

**4. Recommended Implementation Order**

1. Finalize protocol/session/transport boundaries.
2. Add testable HTTP request handling logic without starting a real server if possible.
3. Add one-shot localhost HTTP transport in `http.rs`.
4. Add HTTP-focused tests for method/path/auth/body/request-id/error behavior.
5. Only after review, expose a command-facing API.
6. Only after backend proof, consider minimal UI or userscript handoff.
7. Separately decide how to handle legacy stored Suno credential settings.

**5. Inconsistencies, Risks, Open Questions**

- Docs say Rust should never own Suno auth, but current app has stored `suno_authorization`, `suno_browser_token`, and `suno_device_id`. The review doc says to keep that legacy path for now; the bridge should not expand it.
- Protocol YAML uses `start_seconds` / `end_seconds`, while the newer implementation request says userscript compact fields should be `start_s` / `end_s`. This is intentionally not changing docs yet, but it is a doc/code drift to track.
- Need to choose an HTTP implementation dependency before networking. Prior review suggested `tiny_http` for a small sync proof, but this has not been approved yet.
- CORS/preflight behavior is not specified in the protocol doc, but browser userscripts may need `OPTIONS` handling depending on how they POST.
- Concurrency policy is not fully defined. For proof, isolated per-session transport is cleanest; later command integration may need to reject or manage concurrent bridge starts.
- Token/request id must not be logged in full.
- Timeout/shutdown on every path needs tests, because this is where one-shot servers often get messy.

**6. Small Reviewable Commit Sequence**

1. `Refine bridge protocol boundaries`
   - Keep current enum/session/config shape.
   - Add any missing tests for compact serde fields and limits.

2. `Add HTTP request parser and validator`
   - Pure/testable helpers for method, path, bearer header, body size, JSON parsing.
   - No live socket yet if avoidable.

3. `Add one-shot HTTP transport`
   - Bind `127.0.0.1:0`.
   - Accept exactly one request.
   - Timeout/shutdown behavior.
   - Still no `lib.rs` integration.

4. `Add HTTP transport tests`
   - Valid POST.
   - Wrong method/path.
   - Missing/wrong bearer token.
   - Request id mismatch.
   - Oversized body.
   - Timeout path if practical without flakiness.

5. `Expose bridge API for future integration`
   - Small internal API returning endpoint/session/result types.
   - Still not registered as a Tauri command unless separately approved.

I made no file changes.



## Decisions

- HTTP implementation only
- Keep disconnected from lib.rs
- No UI
- No FFmpeg
- Request parser before live server
- tiny_http (approved)

## Risks

- Legacy Suno credentials remain
- CORS deferred
- One-shot shutdown requires tests

## Approved Commit Sequence

1. Request parser
2. HTTP transport
3. Tests
4. API exposure
5. Integration (future)
