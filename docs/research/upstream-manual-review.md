# Upstream Suno Manual Review

## Purpose

This document evaluates claims from the external `MANUAL_SUNO_OPERATIVO.md` against the architecture and security contracts of `suno_mv`.

The upstream manual is treated as time-bound operational research, not as an authoritative Suno API specification.

## Review policy

Each claim must be recorded with:

- the claim as stated by the upstream source
- whether it has been independently observed in `suno_mv`
- whether `suno_mv` adopts it
- the reason for rejection or deferral
- the last verification date
- the risk that Suno may change the behavior

## Review table

| Claim | Observed in `suno_mv` | Adopt | Rejection or deferral reason | Verified on | Change risk |
|---|---:|---:|---|---|---|
| Browser-owned authentication is more reliable than extracting Chrome cookies | Partly | Yes | Aligns with the Browser Bridge contract and avoids exporting browser credentials | 2026-07-17 | Medium |
| A temporary Clerk token can be obtained in the Suno page context | No | No | Authentication material must not cross the browser boundary | Not verified | High |
| Suno feed responses contain reusable song metadata | Yes | Yes | Metadata is already normalized by the Rust backend; future collection should happen in the browser adapter | 2026-07-17 | High |
| Suno audio and cover assets can be resolved from metadata or CDN URLs | Yes | Yes | Required for MV generation, with fallback and validation | 2026-07-17 | Medium |
| Model labels map to stable internal `mv` identifiers | No | Research only | Internal identifiers are undocumented and may change without notice | Not verified | Very high |
| Omitting `mv` selects an old model | No | No | Not independently verified and generation is outside the current application scope | Not verified | Very high |
| One generation request always returns two variants | No | Research only | Not required for current MV generation features | Not verified | High |
| HTTP 403 from an audio URL means generation is still pending | No | Research only | Status semantics must be verified before implementation | Not verified | High |
| Randomized delays and warm windows should be used to sustain automated generation | No | No | Rate-limit or challenge circumvention is outside project scope | Not verified | Very high |
| CAPTCHA or Turnstile tokens should be captured and reused | No | No | Conflicts with security boundaries and service protections | Not verified | Very high |
| Completed, non-trashed tracks above a duration threshold are useful dataset candidates | No | Deferred | Dataset production is not currently a `suno_mv` responsibility | Not verified | Medium |

## Adoption boundary

The project may adopt:

- metadata field observations
- asset selection and fallback rules
- completion-state observations after independent verification
- model identifiers as dated research notes only
- payload schemas that contain song data but no browser credentials

The project must not adopt:

- exporting cookies, session tokens, authorization headers, browser tokens, or device identifiers
- storing Suno authentication in desktop settings
- CAPTCHA token collection or reuse
- deliberate evasion of rate limits or anti-automation controls

## Verification rule

Any observation about an undocumented Suno endpoint must include a verification date and must be revalidated before code depends on it.
