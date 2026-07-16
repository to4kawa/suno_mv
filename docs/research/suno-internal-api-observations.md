# Suno Internal API Observations

## Status

Research notes only. Suno internal endpoints are undocumented and are not project contracts.

## Current repository observation

The current Rust backend contains a legacy metadata path that requests:

```text
https://studio-api-prod.suno.com/api/feed/?page={page}
```

and supplies desktop-held authorization, browser-token, and device-id settings.

This path predates the Browser Bridge security contract and is a migration target, not the preferred architecture.

## External upstream claims

The reviewed upstream manual reports the following endpoints and behaviors:

| Area | Reported endpoint or behavior | Local status |
|---|---|---|
| Generation | `POST /api/generate/v2/` | Not verified; outside current product scope |
| Library | `GET /api/feed/v2?page=N` | Variant not verified; repository currently uses another feed path |
| Billing | `GET /api/billing/info/` | Not verified; outside current product scope |
| Assets | CDN URLs derived from a clip UUID | Partly observed through live Suno metadata and asset fallback code |
| Pending asset | Small HTTP 403 response before completion | Not verified |

## Architectural use

Undocumented API knowledge may be used only inside a browser-side adapter that runs in the authenticated Suno page context.

The desktop application may receive normalized results through Browser Bridge. It must not receive or persist:

- cookies
- Clerk or session tokens
- authorization headers
- browser tokens
- device identifiers
- Google credentials

## Observation record template

Use this template for every new finding:

```yaml
observation:
  name: ""
  source: "manual | browser-devtools | repository | issue"
  observed_behavior: ""
  endpoint: ""
  request_fields: []
  response_fields: []
  verified_on: "YYYY-MM-DD"
  verified_by: ""
  reproducible: false
  code_dependency: false
  change_risk: "low | medium | high | very_high"
  notes: ""
```

## Implementation rule

Before adding an undocumented endpoint to production code:

1. Verify it in the browser context.
2. Record the date and minimum required fields.
3. Define a normalized credential-free payload.
4. Update or add a contract.
5. Add failure and fallback behavior.
6. Keep the undocumented response shape out of UI and FFmpeg modules.
