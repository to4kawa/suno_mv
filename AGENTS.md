\# AGENTS.md



\## Project



Suno\_mv is a desktop application for generating music videos from Suno songs.



The project consists of:



\- Rust backend

\- Tauri

\- React frontend

\- FFmpeg integration



\---



\## Development Principles



Do not break existing workflows.



Prefer incremental changes.



Implement small reviewable commits.



Avoid unrelated refactoring.



\---



\## Security



Never expose:



\- browser cookies

\- authentication tokens

\- session values



Browser authentication belongs to the browser.



\---



\## Architecture



See:



docs/architecture/

docs/protocol/

docs/adr/



Architecture documents are the source of truth.



\---



\## Development Flow



1\. Read docs/

2\. Propose implementation plan

3\. Wait for approval if architecture changes

4\. Implement

5\. Run tests

6\. Summarize changes



\---



\## Coding Style



Prefer pure functions.



Separate protocol parsing.



Keep UI and backend loosely coupled.



Avoid global mutable state.



\---



\## Testing



Prefer unit tests for protocol parsing.



Do not remove existing functionality.



Regression safety is important.

