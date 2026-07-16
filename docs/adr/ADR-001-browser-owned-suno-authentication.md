# ADR-001: Browser-Owned Suno Authentication

## Status

Accepted

## Date

2026-07-17

## Context

`suno_mv` needs song metadata and asset references from pages that may require an authenticated Suno session.

Previous approaches placed Suno authorization, browser-token, and device-id values in desktop application settings and used them from the Rust backend. This creates unnecessary credential handling and couples the desktop application to undocumented authentication details.

Modern browser protections also make external cookie extraction and automated third-party login brittle.

## Decision

The authenticated browser owns all Suno authentication.

A browser-side adapter may use the session already available to the Suno page. It sends only normalized, credential-free results to the desktop application through the localhost Browser Bridge.

The desktop application must not receive, store, or log:

- browser cookies
- authorization headers
- Clerk or session tokens
- browser tokens
- device identifiers
- Google credentials

Browser Bridge sessions remain localhost-only, short-lived, request-scoped, and authenticated with project-generated session credentials unrelated to Suno authentication.

## Consequences

### Positive

- Suno credentials remain inside the browser security boundary.
- The desktop application does not depend on Chrome cookie storage or OAuth automation.
- Undocumented Suno response formats can be isolated in a browser adapter.
- Rust, UI, and FFmpeg modules consume a stable normalized payload.

### Negative

- A browser-side integration is required for authenticated metadata.
- The adapter must be maintained when Suno changes its page or internal APIs.
- Some metadata may be unavailable when the browser integration is not active.

## Implementation guidance

- Use `contracts/browser_bridge.spec.yaml` for transport.
- Use `contracts/suno_metadata_payload.spec.yaml` and `contracts/suno_asset_payload.spec.yaml` for result payloads.
- Keep direct Suno endpoint details out of renderer and FFmpeg code.
- Record undocumented API observations under `docs/research/` with verification dates.

## Rejected alternatives

### Desktop-held Suno credentials

Rejected because it expands the credential boundary and conflicts with the Browser Bridge security contract.

### Cookie extraction from Chrome

Rejected because it is brittle, browser-version-dependent, and unnecessary.

### Automated Google login in a controlled browser

Rejected because it is fragile and may trigger provider anti-automation protections.
