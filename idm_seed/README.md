# idm_seed

Turns a drawn spectrogram into an IDM-style audio seed (WAV) for Suno Cover / Extend.
Partials are found in the drawing, moved toward integer multiples of f0 only **partially**
(some are left alone and detuned on purpose), then played with additive synthesis.
Python 3 + numpy only (Pillow only for PNG input).

```bash
python3 -m idm_seed demo --out out/                       # A/B at s = 0..1, plus REF (fully harmonic)
python3 -m idm_seed long --minutes 10 --mono --out seed.wav
python3 -m idm_seed render --input art.png --notes notes.json --strength 0.5 --out seed.wav
python3 -m idm_seed render --input art.png --out timbre.wav   # no notes: follows the drawing's own pitch contour
python3 -m pytest idm_seed/tests -q -p no:cacheprovider
```

- `notes.json`: `[{"start": sec, "dur": sec, "midi": 57, "vel": 0.9}, ...]` (midi may be fractional).
- Melody makers (`--style`): `classic` (original: fixed A minor pentatonic, 120 bpm, jumpy) and
  `motif` (stepwise, 2-bar motif + answer, cadence on tonic/fifth, form A A B A C C B A) with
  `--root` (MIDI tonic, 57 = A3), `--scale` (minor, major, dorian, phrygian, minor/major_pentatonic,
  whole_tone), `--bpm`, `--leap` (0 = fully stepwise). `--seed` gives a different melody each time.
  `python3 -m idm_seed melody --style motif --minutes 3 --seed 5 --root 62 --scale dorian --out notes.json`
  writes only the notes; `long ... --notes-out notes.json` renders audio and keeps the notes.
  MIDI: add `--midi-out x.mid` to `melody` or `long`, or convert any notes JSON with
  `python3 -m idm_seed tomidi --notes notes.json --bpm 120 --out x.mid` (Standard MIDI File, format 0,
  480 ticks/beat; the tempo is written into the file so the beat grid matches; pitches are rounded
  to integers; meant for monophonic melodies).
  Hand-written or other-tool melodies (e.g. from MIDI converted to this JSON) work the same way.
- Output length always comes from the input (last note end + tail, or frames x frame_sec + tail).
- `--strength` 0 = leave the drawing as is, 1 = snap (never fully clean). `--variation` and `--ramp`
  make every note differ (strength jitter, messy-to-tidy morph inside a note).
- `--gs gs.npy` enables the novelty filter against the mean spectrum of GS sound-source renders.
- Input PNG: x = time, y = frequency (bottom = low, linear 0..8 kHz by default), brightness squared = magnitude.

## Workflow tools (record, rank, remember)

```bash
# every long/render run writes seed.json next to seed.wav: recipe, PCM hash, metrics, environment
python3 -m idm_seed verify --sidecar seed.json         # WAV unchanged? recipe still rebuilds it bit-for-bit?
python3 -m idm_seed analyze --wav seed.wav --notes notes.json   # periodicity / roughness / noise / inharmonicity

# render a grid, score every candidate, keep the best per strength (WAV + sidecar), write ranking.csv
python3 -m idm_seed batch --style motif --minutes 2 --root 57 --scale minor --bpm 110 \
    --grid config.seed=0,1,2,3 --grid config.strength=0.3,0.5,0.7 --keep 1 --mono --out cands/

# remember what Suno did with them (append-only library.jsonl; notes may be Japanese)
python3 -m idm_seed library add --dir cands/ --tags A_minor
python3 -m idm_seed library note --id d6e0 --rating 4 --mode cover --text "structure carried"
python3 -m idm_seed library list --sort rating
python3 -m idm_seed library stats    # after >= 5 rated seeds: rank correlation metric vs rating + suggested weights
```

- The score is a **playability index** (clear pitch, little beating, little noise), not taste; its
  weights are uncalibrated. Ranking is inside groups (`--group-by`, default `config.strength`) so strength
  stays your creative choice. Feed `library stats` suggestions back with `batch --weights w.json`.
- Grid paths address the recipe (`config.*`, `melody.seed`, `melody.options.scale`, `input.seed`).
- `verify` also reports `same_numpy`: another numpy/Python build may differ in the last bits.
- Python syntax is kept compatible with 3.9 (checked at parse level only); numpy is the only dependency.

Known limits: residual (non-harmonic) content is dropped by additive synthesis; f0 is the lowest
significant peak (a missing fundamental is not handled); results were judged with objective metrics
and the owner's listening, Suno's behaviour is not modelled.
