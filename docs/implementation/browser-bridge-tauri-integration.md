\# Browser Bridge Tauri Integration



\## Goal



Integrate the Browser Bridge into the existing Tauri application without changing the Browser Bridge architecture.



The Browser Bridge implementation is already complete as an isolated module.

This phase connects it to the application.



\---



\## Scope



\- lib.rs

\- Tauri command registration

\- Minimal frontend invocation



\---



\## Requirements



\### Tauri



Register a command that creates a Browser Bridge session.



Example flow:



UI



↓



invoke()



↓



create\_browser\_bridge()



↓



start one-shot transport



↓



wait for browser request



↓



return result



\---



\### Browser Bridge



Reuse the existing implementation.



Do not redesign:



\- protocol

\- session

\- transport

\- parser



\---



\### Frontend



A temporary proof UI is acceptable.



Requirements:



\- Start Browser Bridge

\- Display waiting state

\- Display localhost endpoint

\- Display request\_id

\- Display completion/error



No production UX is required.



\---



\## Out of Scope



\- FFmpeg integration

\- SRT generation

\- Userscript improvements

\- Rendering pipeline

\- Existing MV workflow changes



\---



\## Definition of Done



\- Browser Bridge can be started from Tauri.

\- Browser Bridge accepts one localhost request.

\- Parsed payload is returned to the frontend.

\- Existing application functionality remains unchanged.



\---



\## Expected Commit Sequence



\### Commit #1



Register Browser Bridge API in lib.rs.



\### Commit #2



Expose Tauri command.



\### Commit #3



Add temporary proof UI.



\### Commit #4



Manual localhost proof.



\---



\## Follow-up



After this phase:



Browser



↓



Userscript



↓



Browser Bridge



↓



Tauri



↓



Frontend



will be fully connected.



The next implementation phase will connect the Browser Bridge to the MV generation pipeline.

