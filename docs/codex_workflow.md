# Codex Work Log and Handoff Notes

This document records the work style expected for this repository: confirm assumptions, inspect the existing design, define evaluation criteria before implementation, keep failed cases visible, and leave the project in a commit-ready state.

## Repository context reviewed

- `README.md` describes an Electron desktop app that creates MP4 music videos from Suno song URLs.
- `src/main.js`, `src/preload.js`, `src/renderer.js`, and `public/index.html` implement the Electron UI flow.
- `src/suno_mv.js` is a CLI-style implementation that downloads Suno audio/cover art and invokes FFmpeg.
- `package.json` previously only exposed `npm start`; there was no automated test command.

## Requirement interpretation

The request is process-oriented rather than a new user-facing feature. The smallest useful implementation is therefore to make an existing seam testable, add automated evaluation cases, and document what was and was not verified.

## Options considered

### Adopted: extract pure helpers from the CLI and test them

Why adopted:

- It creates measurable behavior without launching Electron, downloading Suno assets, or invoking FFmpeg.
- It keeps the implementation minimal and avoids broad integration work.
- It makes URL parsing, path construction, CDN URL construction, and FFmpeg command construction independently reviewable.

### Not adopted: rewrite the Electron main-process generation flow

Why not adopted:

- The request did not ask for a user-facing UI change.
- Reworking IPC generation and settings would expand scope and create more unverified behavior.
- It would likely require manual Electron and FFmpeg checks that are not safe to claim in this environment.

### Not adopted: add an end-to-end FFmpeg/Suno network test

Why not adopted:

- It depends on external CDN availability, a valid public Suno URL, and local FFmpeg installation.
- It may download copyrighted or user-provided media.
- It would be slow and flaky compared with deterministic unit tests.

## Evaluation criteria defined before implementation

A change is considered acceptable when all of the following are true:

1. `src/suno_mv.js` can still be executed directly as a CLI.
2. Importing `src/suno_mv.js` from tests does not run downloads or FFmpeg.
3. Suno song IDs can be extracted from standard `/song/<id>` URLs.
4. Invalid URLs are rejected by returning `null` from the pure parser.
5. CDN URL construction sanitizes IDs before embedding them in paths.
6. Output paths are generated under the configured output directory.
7. FFmpeg command construction preserves spaces in paths by quoting input and output paths.
8. `npm test` runs the deterministic checks without Electron, network, or FFmpeg.

## Failure conditions

The work should be treated as failed if any of these occur:

- Importing `src/suno_mv.js` starts a download, exits the process, or invokes FFmpeg.
- The CLI entry point no longer works when `node src/suno_mv.js <Suno Song URL>` is used.
- Tests require network access, Electron, or FFmpeg.
- Tests only cover successful input and omit invalid or edge inputs.

## Implemented changes

- `src/suno_mv.js` now exports pure helper functions and only runs `main()` when executed directly.
- `test/suno_mv.test.js` covers normal, invalid, and edge-oriented behavior for URL parsing and command construction.
- `package.json` now includes `npm test`.

## Test and evaluation cases

Run:

```bash
npm test
```

Covered cases:

- Standard Suno song URL extracts the song ID.
- Non-song and malformed strings return `null`.
- CDN asset URLs are built from a sanitized ID.
- Generated MP3, JPEG, and MP4 paths remain under the output directory.
- FFmpeg command includes the selected resolution and visualizer mode.
- FFmpeg command quotes paths that include spaces.

## Known constraints

- The tests do not validate that Suno CDN URLs are currently reachable.
- The tests do not validate that FFmpeg is installed or that a generated MP4 is playable.
- The tests do not launch the Electron UI.
- The FFmpeg command is still string-based because the existing implementation uses `child_process.exec`; switching to `spawn` with argument arrays would be a separate, safer refactor.

## Unverified items

- Real Suno download behavior with a live song URL.
- Real FFmpeg rendering behavior and output media quality.
- Electron renderer-to-main IPC behavior.
- Windows-specific FFmpeg path behavior used by `src/main.js`.

## Failed examples and handling

These examples are intentionally represented in tests or documented as non-goals:

- `https://suno.com/create` is not a song URL and should not produce a song ID.
- `not a url` is malformed input and should not produce a song ID.
- IDs containing path-like punctuation are sanitized before CDN URL construction.

## User-facing explanation

Users can keep running the app with:

```bash
npm start
```

Maintainers and contributors can now run deterministic checks with:

```bash
npm test
```

## Maintainer-facing explanation

The CLI module now has a safe import boundary. Future work should add tests around pure helpers first, then add integration tests only when the environment can provide explicit sample media, FFmpeg, and permission to perform network downloads.
