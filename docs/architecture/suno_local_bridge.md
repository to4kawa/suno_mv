\# Suno Local Browser Bridge



\## Goal



Prove secure communication between:



```

Authenticated Browser

&#x20;       │

&#x20;       ▼

Userscript

&#x20;       │ JSON

&#x20;       ▼

Temporary Localhost Bridge

&#x20;       │

&#x20;       ▼

Rust Backend

&#x20;       │

&#x20;       ▼

FFmpeg

```



This project is \*\*not\*\* about SRT generation.



SRT is only the first proof payload.



\---



\## Lifecycle



```

command



↓



localhost server starts



↓



request\_id generated



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



\---



\## Security



Never transfer:



\- Cookie

\- Authorization

\- Browser Token

\- Session

\- Google credentials



Transfer only processed data.



\---



\## Success Criteria



\- Browser remains authentication owner

\- Rust never owns authentication

\- One-shot localhost communication

\- Automatic shutdown

\- Existing MV generation remains functional



\---



\## Future Payloads



\- aligned\_lyrics

\- metadata

\- assets

\- account

\- song

