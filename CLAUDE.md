# CLAUDE.md

Claude Code notes for suno_mv. Agent workflow rules live in [AGENTS.md](AGENTS.md); the contracts and ADRs are the source of truth. Read them before changing anything.

## Current direction

The project is being redirected from the MV maker to a new purpose: **`idm_spectrogram_filter`**. It takes a freely drawn (image-generation style) spectrogram and deliberately *incompletely* corrects physically impossible harmonic relationships to produce IDM-style timbres ("broken but controlled"). Phase is left to a later stage (additive synthesis or vocoder).

- Spec (source of truth for this task): [docs/tasks/idm_spectrogram_filter.yaml](docs/tasks/idm_spectrogram_filter.yaml)
- Never fully clean the harmonic relationships; keeping glitch and intermediate states is the point (`correction_strength` ~0.3–0.7).
- **Undecided (ask the user, do not assume):** implementation language/stack, where the new code lives (design under review), and what `gs_spectrum` ("GS" source) concretely is.

## Existing code: Suno MV Maker (kept as-is; do not modify unless asked)

The owner decided to keep this code. Do not delete or refactor it as part of the new task; add new work alongside it.

Suno MV Maker: a Tauri desktop app. The renderer (`public/`, plain HTML/JS) calls Rust through Tauri IPC (`invoke("generate_mp4")`). Rust (`src-tauri/`) resolves the Suno song ID, builds the FFmpeg argv, and runs FFmpeg to produce an MP4 visualizer. A local browser bridge (`src-tauri/src/bridge/`) lets the browser, which owns authentication, talk to the desktop app.

## Layout

- `public/` renderer (`index.html`, `renderer.js`)
- `src-tauri/src/` Rust backend (`lib.rs`, `main.rs`, `bridge/`)
- `src-tauri/tests/` Rust integration tests (`bridge_protocol.rs`)
- `test/` Node tests for the renderer
- `contracts/browser_bridge.spec.yaml` bridge contract
- `docs/` `adr/`, `architecture/`, `protocol/`, `implementation/`, `reviews/`

## Commands

```bash
npm install
npm test                                              # node --test (renderer)
cargo check --manifest-path src-tauri/Cargo.toml
cargo test  --manifest-path src-tauri/Cargo.toml
npm start                                             # tauri dev (needs Tauri deps + FFmpeg on PATH)
```

Per AGENTS.md, do not run long builds or tests unless asked. Network in cloud sessions may block crate/npm downloads, so `cargo` and `npm install` can fail there; report that as unverified, not as passing.

## Rules that must not be broken

- **Bridge security** (contract): the desktop side never receives, stores, or logs browser credentials, cookies, or Authorization headers. Authentication stays in the browser.
- FFmpeg is run with an argv array, never a shell string. Resolution and visualizer values are allowlisted before execution.
- Keep protocol parsing separate from transport. Avoid global mutable state.
- If an implementation conflicts with a contract or architecture doc: stop, explain the conflict, do not guess.

## Working style

- Small, reviewable commits; no unrelated refactoring.
- For architecture changes: summarize understanding, propose a plan, wait for approval.
- After implementing, summarize what was done and recommend manual verification steps.
