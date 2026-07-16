# Suno Model Identifier Observations

## Policy

Suno model identifiers are undocumented implementation details. They must be recorded as dated observations, never as permanent project constants without revalidation.

## Upstream-reported mapping

The external manual reviewed on 2026-07-17 reports:

| UI label | Reported internal identifier | Local verification | Project use |
|---|---|---:|---|
| v5.5 | `chirp-fenix` | No | Research only |
| v5 | `chirp-crow` | No | Research only |
| v4 | `chirp-v4` | No | Research only |
| v2 | `chirp-v2` | No | Research only |

The same source claims that omitting the model identifier from a generation request may select `chirp-v2`. This has not been independently verified by `suno_mv`.

## Storage guidance

When model information is received through Browser Bridge, preserve both forms when available:

```yaml
model:
  label: "v5.5"
  identifier: "chirp-fenix"
  observed_on: "2026-07-17"
  source: "upstream-manual"
  verified: false
```

## Rules

- Do not infer an internal identifier from a UI label.
- Do not use an unverified mapping to initiate generation.
- Preserve unknown identifiers rather than rejecting otherwise usable song metadata.
- Display the UI label to users when available.
- Keep historical identifiers for diagnostics and dataset provenance.
- Reverify all mappings before implementing generation features.
