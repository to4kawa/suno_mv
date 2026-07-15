\# ADR-0001: Local Browser Bridge



\## Status



Accepted



\## Context



Suno authentication relies on a browser-based Google login.



The desktop application should not store or manage browser credentials.



The existing Userscript already runs inside an authenticated browser session.



We need a mechanism to transfer processed data from the authenticated browser to the local application.



\## Decision



Introduce a short-lived localhost bridge.



The desktop application:



\- starts a temporary localhost HTTP server

\- generates a temporary request\_id

\- generates a temporary bearer token

\- waits for one authenticated payload

\- shuts down immediately after completion



The browser Userscript:



\- performs authenticated requests

\- converts responses into structured JSON

\- sends only processed data

\- never sends browser credentials



\## Consequences



Advantages



\- Browser credentials remain inside the browser.

\- Rust never stores Suno authentication.

\- Protocol can later support metadata, assets, lyrics and other providers.

\- Easy to test.



Disadvantages



\- Requires localhost communication.

\- Browser Userscript must understand the bridge protocol.



Future



This protocol may become a generic Browser Bridge for additional services.

