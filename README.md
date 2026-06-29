# Suno MV Maker – Tauri App

Suno の曲 URL から `clip_id` を抽出し、Cookie 付きの metadata 取得で `audio_url` / `image_url` / `video_url` を調査・解決して、FFmpeg で MV 用 MP4 を生成する軽量デスクトップアプリです。

> Suno の内部 API は固定仕様として扱いません。`src-tauri/src/main.rs` は複数の候補 endpoint を順番に試し、metadata 内の `audio_url` と `image_url` を優先します。CDN URL の推測は画像フォールバックに限定しています。

## Features

- Suno URL 入力
- URL から `clip_id` 抽出
- `SUNO_COOKIE` または UI 入力 Cookie による曲 metadata 取得
- `audio_url` / `image_url` / `video_url` の抽出
- カバー画像プレビュー
- ドラッグ＆ドロップ / ファイル選択で画像差し替え
- 解像度選択
- visualizer 種別選択（Spectrum / Waveform）
- Rust 側で FFmpeg 実行・ログ転送・キャンセル
- Playwright worker は `workers/playwright-suno/` に隔離予定（認証セッション取得・Network 調査用）

## Requirements

- Rust / Cargo
- Tauri CLI (`cargo install tauri-cli`)
- FFmpeg (`ffmpeg` が PATH にあること)
- 必要に応じて Suno の Cookie

## Configuration

Cookie は以下のどちらかで渡せます。

1. 環境変数

```bash
export SUNO_COOKIE='...'
```

2. アプリ画面の `SUNO_COOKIE` 欄

## Development

```bash
cargo tauri dev
```

型チェックのみ:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## Build

```bash
cargo tauri build
```
