\# Browser Bridge HTTP Transport



\## Goal



Implement the one-shot localhost transport defined by ADR-0001.



\## Scope



\- transport.rs

\- http.rs



\## Requirements



\- Bind only to 127.0.0.1

\- Use a dynamic port

\- Accept only POST /v1/result

\- Require Bearer authentication

\- Validate request\_id

\- Enforce body size limit

\- Timeout after 60 seconds

\- Accept exactly one request

\- Shut down after success, timeout, or error



\## Out of Scope



\- lib.rs integration

\- UI

\- FFmpeg

\- Userscript changes



\## Definition of Done



\- HTTP transport compiles

\- Unit tests pass

\- No application wiring

