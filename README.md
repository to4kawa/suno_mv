# Suno MV Maker – Tauri App

Suno MV Maker is a desktop app that starts as a Tauri window and calls a Rust backend through Tauri IPC to generate a basic MP4 music visualizer from a Suno song URL.

## PR #4 goal

PR #4 establishes the minimum end-to-end architecture after removing Electron:

- Tauri window startup from `npm start` / `npm run dev`
- Renderer-to-Rust IPC with `invoke("generate_mp4")`
- Rust backend command registration with `#[tauri::command] generate_mp4`
- Rust-side Suno ID extraction, resolution allowlist, visualizer allowlist, path construction, FFmpeg args construction, and FFmpeg execution
- FFmpeg execution through an argv array instead of a shell command string

## Features

- Paste a Suno song URL
- Preview the default Suno cover image
- Drag and drop a custom cover image
- Generate an MP4 via Rust + FFmpeg

## Requirements

- [Node.js](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri system dependencies](https://tauri.app/start/prerequisites/)
- [FFmpeg](https://ffmpeg.org/) installed and available on `PATH`

## Install

```bash
npm install
```

## Run the Tauri window

```bash
npm start
```

or:

```bash
npm run dev
```

The development command starts a local static server for `public/` and then launches the Tauri window. The renderer calls `invoke("generate_mp4")`, which is registered by the Rust Tauri builder.

## Test and checks

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm install
npm test
npm start
```

## Current verification status

Verified in this change:

- `generate_mp4` is registered as a Tauri IPC command in the Rust source.
- The renderer calls `invoke("generate_mp4")`.
- FFmpeg arguments are built as an argv array, not a shell string.
- Resolution and visualizer values are allowlisted before FFmpeg execution.
- Node renderer tests pass.

Not yet fully verified:

- `cargo check`, `cargo test`, `npm install`, and `npm start` require downloading crates/npm packages or an installed Tauri CLI; the current CI/container network returned 403 responses while fetching dependencies.
- Tauri window launch must be re-run in an environment with Tauri dependencies available.
- Live Suno CDN MP3/cover downloads can fail depending on Suno availability, network restrictions, or CDN behavior.
- Real MP4 generation with FFmpeg requires a valid live Suno URL and local FFmpeg installation.
- Tauri packaging/bundling has not been validated yet.
