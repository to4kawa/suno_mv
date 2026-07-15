# Suno Local Browser Bridge

## Goal

Prove secure communication between:

```
Authenticated Browser
        │
        ▼
Userscript
        │ JSON
        ▼
Temporary Localhost Bridge
        │
        ▼
Rust Backend
        │
        ▼
FFmpeg
```

This project is **not** about SRT generation.

SRT is only the first proof payload.

---

## Lifecycle

```
command

↓

localhost server starts

↓

request_id generated

↓

token generated

↓

browser opens Suno page

↓

Userscript fetches authenticated data

↓

POST JSON

↓

Rust validates payload

↓

Rust processes payload

↓

shutdown
```

---

## Security

Never transfer:

- Cookie
- Authorization
- Browser Token
- Session
- Google credentials

Transfer only processed data.

---

## Success Criteria

- Browser remains authentication owner
- Rust never owns authentication
- One-shot localhost communication
- Automatic shutdown
- Existing MV generation remains functional

---

## Future Payloads

- aligned_lyrics
- metadata
- assets
- account
- song