\# Codex Review



\## Context



Initial implementation planning for Local Browser Bridge proof.



\## Codex Summary



\*\*1. Existing Architecture Summary\*\*



The app is a small Tauri desktop app:



\- Frontend: \[public/index.html](C:/Users/to4ka/OneDrive/ドキュメント/Suno\_mv/suno\_mv/public/index.html), \[public/renderer.js](C:/Users/to4ka/OneDrive/ドキュメント/Suno\_mv/suno\_mv/public/renderer.js)

\- Backend: mostly centralized in \[src-tauri/src/lib.rs](C:/Users/to4ka/OneDrive/ドキュメント/Suno\_mv/suno\_mv/src-tauri/src/lib.rs)

\- Tauri commands currently include settings, FFmpeg detection/testing, folder opening, Suno metadata fetch, and MP4 generation.

\- Metadata fetching currently happens in Rust via `fetch\_suno\_metadata`, with optional stored Suno auth/browser token/device id settings.

\- Rendering uses fetched or fallback metadata/audio/cover URLs, then FFmpeg builds the final MP4.

\- Architecture docs define a new one-shot localhost bridge where the authenticated browser owns auth, the userscript fetches Suno data, and Rust receives only processed JSON.



Important inconsistency: the docs say Rust should never own Suno authentication, but the current app already has `suno\_authorization`, `suno\_browser\_token`, and `suno\_device\_id` stored in settings/UI. I would not edit docs yet, but implementation should avoid expanding that path and should probably deprecate it later.



\*\*2. Integration Points\*\*



\- Add a new Tauri command, likely `start\_suno\_bridge`, invoked from the frontend.

\- The command starts a temporary `127.0.0.1:0` HTTP server, generates:

&#x20; - `request\_id`

&#x20; - bearer token

&#x20; - dynamic port

\- The frontend receives bridge connection info and opens/copies a Suno/userscript handoff URL or displays the local endpoint/token/request id for proof testing.

\- Userscript posts to:



```text

POST http://127.0.0.1:{dynamic\_port}/v1/result

Authorization: Bearer {token}

```



\- Backend validates:

&#x20; - method/path

&#x20; - localhost bind only

&#x20; - bearer token

&#x20; - `request\_id`

&#x20; - `protocol\_version`

&#x20; - provider/type

&#x20; - body size <= 2 MB

&#x20; - timing count <= 5000

\- On valid payload, backend converts `aligned\_lyrics.timings` into the internal proof output, probably first as a received payload/result displayed in UI, then later as an SRT or render input.

\- Server shuts down after success, timeout, or error.



\*\*3. Files That Will Need Modification\*\*



\- \[src-tauri/Cargo.toml](C:/Users/to4ka/OneDrive/ドキュメント/Suno\_mv/suno\_mv/src-tauri/Cargo.toml)

&#x20; - Add minimal HTTP/server and randomness dependencies if not using std-only.

&#x20; - Likely candidates: `tiny\_http` or `axum`/`tokio`; for smallest proof, `tiny\_http` plus `rand`/`uuid` is enough.

\- \[src-tauri/src/lib.rs](C:/Users/to4ka/OneDrive/ドキュメント/Suno\_mv/suno\_mv/src-tauri/src/lib.rs)

&#x20; - Register the new command.

&#x20; - Ideally move bridge code into modules instead of adding more to the large file.

\- \[public/index.html](C:/Users/to4ka/OneDrive/ドキュメント/Suno\_mv/suno\_mv/public/index.html)

&#x20; - Add a small bridge proof panel/button/status area.

\- \[public/renderer.js](C:/Users/to4ka/OneDrive/ドキュメント/Suno\_mv/suno\_mv/public/renderer.js)

&#x20; - Invoke bridge command.

&#x20; - Render bridge status/result.

\- \[test/renderer.test.js](C:/Users/to4ka/OneDrive/ドキュメント/Suno\_mv/suno\_mv/test/renderer.test.js)

&#x20; - Add basic frontend command wiring tests.

\- Potentially \[README.md](C:/Users/to4ka/OneDrive/ドキュメント/Suno\_mv/suno\_mv/README.md)

&#x20; - Only if proof usage needs developer instructions. Not architecture docs unless the credential inconsistency is explicitly resolved.



\*\*4. New Modules That Should Be Introduced\*\*



Under `src-tauri/src/`:



\- `bridge/mod.rs`

&#x20; - Public command-facing API.

&#x20; - Starts server and returns bridge session info/result.

\- `bridge/protocol.rs`

&#x20; - Pure structs and validation for protocol v1.

&#x20; - Payload types:

&#x20;   - `BridgePayload`

&#x20;   - `AlignedLyricsPayload`

&#x20;   - `TimingRecord`

&#x20; - Unit tests should live here.

\- `bridge/server.rs`

&#x20; - One-shot localhost HTTP server.

&#x20; - Handles timeout, max body, auth header, endpoint/method validation, shutdown.

\- `bridge/security.rs` or helpers inside `protocol.rs`

&#x20; - Token generation/masking, forbidden credential-field detection if wanted.

\- Optional later: `bridge/srt.rs`

&#x20; - Convert aligned timings to SRT once the proof moves beyond “receive and validate”.



This keeps with AGENTS.md: separate protocol parsing, prefer pure functions, avoid global mutable state.



\*\*5. Risks\*\*



\- Existing credential settings conflict with the new security direction. The proof should not use them.

\- Blocking server lifecycle from a Tauri command needs care. A command that waits up to 60 seconds may be acceptable for proof, but the UI needs clear “waiting” state.

\- Dynamic localhost ports can be blocked by firewall/security software.

\- Userscript/browser CORS behavior needs explicit response headers and possibly handling `OPTIONS`.

\- One-shot shutdown must happen on all paths: success, invalid request, timeout, and body limit failure.

\- Token/request id must not be logged in full.

\- If multiple bridge attempts are started concurrently, behavior should be defined. For proof, reject concurrent sessions or keep each session isolated.

\- Protocol YAML says `request\_id` is required in payload; docs also say bearer token is generated. Both should be validated, not treated as interchangeable.



\*\*6. Suggested Implementation Order\*\*



1\. Add pure protocol structs and validators with Rust unit tests.

2\. Add token/request id generation and masked logging helpers.

3\. Add one-shot localhost server with a testable handler boundary.

4\. Add Tauri command `start\_suno\_bridge`.

5\. Add minimal UI proof panel:

&#x20;  - start bridge

&#x20;  - show waiting state

&#x20;  - show endpoint/request id/token hint safely

&#x20;  - show received aligned lyrics summary

6\. Add a local manual test fixture command or curl instructions for proof.

7\. Wire userscript expectations only after backend proof works.

8\. Decide separately whether to deprecate/remove current stored Suno API credentials.



\*\*7. Small Reviewable Commit Plan\*\*



1\. `Add bridge protocol validation`

&#x20;  - New `bridge/protocol.rs`

&#x20;  - Payload structs

&#x20;  - Validation tests for required fields, limits, request id, timing shape



2\. `Add one-shot localhost bridge server`

&#x20;  - New `bridge/server.rs`

&#x20;  - Bind `127.0.0.1:0`

&#x20;  - `POST /v1/result`

&#x20;  - bearer auth

&#x20;  - body limit

&#x20;  - timeout/shutdown behavior



3\. `Expose bridge through Tauri command`

&#x20;  - Register `start\_suno\_bridge`

&#x20;  - Return session/result DTOs

&#x20;  - Keep full tokens out of logs



4\. `Add bridge proof UI`

&#x20;  - Small panel in `index.html`

&#x20;  - Renderer invokes command and displays status/result

&#x20;  - Frontend tests for command wiring



5\. `Add manual proof docs or README note`

&#x20;  - Example localhost POST payload

&#x20;  - Security note: no cookies/auth/session values

&#x20;  - Only update architecture docs if the team decides how to resolve the existing stored-credential inconsistency.



\## Human Review



\### Accepted



\- Existing architecture summary

\- Integration points

\- Reviewable commit plan



\### Requested Changes



1\. Session / Protocol / Transport separation

2\. request\_id != token

3\. Backend proof before UI

4\. Keep legacy credential path



\## Decision



Approved after requested revisions.

