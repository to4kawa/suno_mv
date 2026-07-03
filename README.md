# Suno MV Maker – Electron App

🎵 Generate a spectrum-visualized music video (MP4) from any [Suno](https://suno.com) song URL using this desktop Electron app.

## ✨ Features

- Paste any Suno song URL
- Preview and replace cover image (drag & drop supported)
- Choose output folder, resolution, and visualizer type
- Generates MP4 with FFmpeg

## 📦 Installation

```bash
npm install
```

## 🚀 Run

```bash
npm start
```

## 🧪 Test

```bash
npm test
```

The automated tests cover deterministic CLI helper behavior only. They do not download Suno assets, launch Electron, invoke FFmpeg, or verify generated video playback.

## 🔧 Requirements

- [Node.js](https://nodejs.org/)
- [FFmpeg](https://ffmpeg.org/) installed and in PATH

## 🧭 Development workflow

See [docs/codex_workflow.md](docs/codex_workflow.md) for the current implementation rationale, accepted and rejected approaches, evaluation criteria, known constraints, unverified items, and failure examples.

## 📄 License

MIT
