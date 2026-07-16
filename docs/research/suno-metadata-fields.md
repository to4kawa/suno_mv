# Suno Metadata Fields

## Purpose

Define the normalized metadata vocabulary used between a browser-side Suno adapter, Browser Bridge, the Rust backend, and MV generation.

This document describes project-level fields. It does not guarantee that Suno uses the same names internally.

## Core song fields

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `id` | string | Yes | Suno song or clip identifier |
| `title` | string | No | Display title |
| `display_name` | string | No | Creator or display name |
| `created_at` | string | No | Source creation timestamp |
| `status` | string | No | Normalized generation or availability state |
| `duration_seconds` | number | No | Track duration in seconds |
| `model_label` | string | No | Human-facing model label, such as `v5.5` |
| `model_identifier` | string | No | Dated internal identifier when observed |

## Text fields

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `lyrics` | string | No | Lyrics intended for display or alignment |
| `prompt` | string | No | Source prompt when available |
| `style` | string | No | Normalized style description |
| `tags` | array of strings | No | Normalized style or genre tags |

## Asset fields

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `audio_url` | string | No | Preferred audio asset |
| `video_url` | string | No | Preferred source video asset |
| `image_url` | string | No | Standard cover image |
| `image_large_url` | string | No | Large cover image |

## Selection rules

Audio:

1. Use a validated `audio_url` from normalized metadata.
2. Otherwise use a validated application fallback.
3. Never construct a URL from an identifier unless that behavior is independently verified and documented.

Cover:

1. Use an explicitly supplied custom image when present.
2. Otherwise prefer `image_large_url`.
3. Otherwise use `image_url`.
4. Otherwise use the application fallback.

Text:

1. Prefer a dedicated `lyrics` field.
2. Treat `prompt` as lyrics only when it passes an explicit lyrics-shape check.
3. Keep style text separate from lyrics.

## Provenance

Every Browser Bridge payload should include provenance metadata:

```yaml
source:
  adapter: "suno-browser"
  observed_on: "YYYY-MM-DD"
  page_url: "https://suno.com/song/..."
  method: "page-data | authenticated-fetch | user-input"
```

No provenance field may contain credentials or raw request headers.
