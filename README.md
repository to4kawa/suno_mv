# Suno MV Maker – Tauri App

This branch is a full migration from the Electron implementation to a Tauri + Rust backend. It is separate from PR #3's incremental/testability line of work, but applies the same rules: small reviewable criteria, explicit unverified items, and no claims that unverified behavior is production-ready.

## Features kept in this migration

- Paste a Suno song URL.
- Preview the default Suno cover image.
- Drag and drop a replacement cover image.
- Generate an MP4 by invoking FFmpeg from the Rust backend.

## Why Electron was removed

Electron-specific files were removed because this PR treats the app shell migration as a full replacement: desktop lifecycle, IPC, and privileged process work now live in Tauri/Rust instead of Electron's main/preload processes. This keeps FFmpeg execution and validation in Rust, avoids maintaining two desktop runtimes during the same migration PR, and makes the replacement boundary explicit for review.

## Requirements

- Node.js and npm.
- Rust toolchain with Cargo.
- FFmpeg installed and available as `ffmpeg` in `PATH`.
- Platform packages required by Tauri for your OS.

## Installation

```bash
npm install
```

## Development run / Tauri startup

```bash
npm start
```

This currently runs the Rust-side startup entrypoint with `cargo run --manifest-path src-tauri/Cargo.toml`. In a desktop-capable Tauri environment, use this step to confirm the Tauri shell startup path before merge; do not mark this PR mergeable until that window launch has been verified.

## Verification commands before merge

This migration PR is not mergeable unless all of the following are completed successfully in the target environment:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
npm install
npm test
npm start
```

`npm start` is the required startup confirmation step. It must open the Tauri window in the target desktop environment before this PR is considered mergeable; if it only reaches the Rust entrypoint, keep startup confirmation marked unverified.

## Security and validation notes

- FFmpeg is invoked by Rust with `Command::new("ffmpeg").args(&args)` and a `Vec<String>` args array, not by joining a shell command string.
- `resolution` is allow-listed to `1280x720` and `1920x1080`.
- `visualizer` is allow-listed to `spectrum` and `showspectrum`.
- Suno real network fetches are implemented against `cdn1.suno.ai` and `cdn2.suno.ai`, but real Suno network acquisition remains **unverified** until tested with live Suno URLs in the target environment.

## Unverified items

- Live Suno MP3 download from `https://cdn1.suno.ai/<id>.mp3` is unverified.
- Live Suno cover download from `https://cdn2.suno.ai/image_large_<id>.jpeg` is unverified.
- End-to-end FFmpeg generation with an actual Suno track is unverified until `npm start` is run with FFmpeg available and a live URL.
- Tauri packaging/bundling is unverified; this PR only requires development startup confirmation.

## Old features that still need to be restored or revalidated

- Output folder selection UI.
- Settings window and persisted FFmpeg/FFprobe path selection.
- Multiple resolution choices in the UI.
- Multiple visualizer choices in the UI.
- FFprobe-based media validation, if it existed in the Electron workflow consumers relied on.
- Packaged app distribution for Windows/macOS/Linux.

## License

MIT
