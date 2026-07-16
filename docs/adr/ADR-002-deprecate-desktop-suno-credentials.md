# ADR-002: Deprecate Desktop-Held Suno Credentials

## Status

Accepted

## Date

2026-07-17

## Context

The current Rust settings model includes:

- `suno_authorization`
- `suno_browser_token`
- `suno_device_id`

The metadata fetch path uses these values to call an undocumented Suno feed endpoint directly from the desktop backend.

ADR-001 establishes that Suno authentication belongs to the browser. The existing settings and direct feed path therefore conflict with the target architecture.

## Decision

Desktop-held Suno credentials and direct authenticated Suno feed access are deprecated.

They will be removed through an incremental migration rather than deleted immediately.

## Migration plan

### Phase 1: Documentation and contracts

- Record the architecture decision.
- Define credential-free metadata and asset payload contracts.
- Mark the existing direct-feed path as legacy in code and documentation.

### Phase 2: Browser Bridge parity

- Implement browser-side retrieval for the metadata required by MV generation.
- Send normalized metadata and asset references through Browser Bridge.
- Add contract validation and credential-field rejection.
- Preserve existing fallback behavior for public page metadata.

### Phase 3: Default path switch

- Make Browser Bridge the preferred authenticated metadata path.
- Stop requesting new Suno credential values in the UI.
- Keep the legacy path available only behind an explicit temporary compatibility option.

### Phase 4: Removal

- Remove the three credential settings.
- Remove direct authenticated feed requests from the Rust backend.
- Remove related logs, UI fields, tests, and migration-only compatibility code.

## Compatibility requirements

During migration:

- existing MV generation must continue to work when public page metadata is sufficient
- custom cover and direct asset fallback behavior must remain available
- no migration step may log stored credential values
- Browser Bridge payloads must be rejected when credential-like fields are present

## Consequences

### Positive

- Aligns implementation with the Browser Bridge contract.
- Reduces sensitive configuration and support burden.
- Isolates Suno-specific undocumented behavior in the browser adapter.

### Negative

- Requires a staged implementation and temporary coexistence of old and new paths.
- Users relying on manually entered credentials will need to migrate.

## Follow-up work

Create implementation issues for:

1. legacy-path annotations and UI deprecation messaging
2. browser metadata adapter
3. metadata payload validation
4. asset payload validation
5. default-path switch
6. credential-setting removal
