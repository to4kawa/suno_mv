"""CLI.  python -m idm_seed demo --out DIR
        python -m idm_seed render --input art.png|spec.npy [--notes notes.json] --strength 0.5 --out seed.wav
"""
from __future__ import annotations

import argparse
import json
import pathlib

import numpy as np

from .draw import drawn_spectrogram
from .midi import write_midi
from .melody import SCALES, make_classic_melody as make_long_melody, make_melody  # noqa: F401
from .pipeline import SeedBuilder, SeedConfig
from .render import Note, write_wav


def load_spectrogram(path, fmax=8000.0, frame_sec=0.032, sr=16000):
    """.npy: [T,F] magnitude on a linear axis 0..sr/2.  .png/.jpg: grayscale, x=time,
    y=frequency (bottom = low, linear 0..fmax), brightness**2 -> magnitude."""
    p = pathlib.Path(path)
    if p.suffix == ".npy":
        S = np.load(p).astype(float)
        return S, np.linspace(0, sr / 2, S.shape[1]), frame_sec
    from PIL import Image
    a = np.asarray(Image.open(p).convert("L"), dtype=float) / 255.0
    S = (a[::-1].T) ** 2
    return S, np.linspace(0, fmax, S.shape[1]), frame_sec


def load_notes(path):
    return [Note(float(n["start"]), float(n["dur"]), float(n["midi"]), float(n.get("vel", 1.0)))
            for n in json.loads(pathlib.Path(path).read_text())]


def load_gs(path, n_bins):
    g = np.load(path).astype(float)
    g = g.mean(axis=0) if g.ndim == 2 else g
    if len(g) != n_bins:
        g = np.interp(np.linspace(0, 1, n_bins), np.linspace(0, 1, len(g)), g)
    return g


DEMO_MELODY = [57, 60, 64, 62, 60, 67, 65, 64, 69, 67, 64, 60, 62, 65, 64, 57]


def cmd_demo(a):
    out = pathlib.Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    S, freqs, fs = drawn_spectrogram(seed=a.seed)
    notes = [Note(i * 0.5, 0.45, m, 0.9) for i, m in enumerate(DEMO_MELODY)]
    for s in (0.0, 0.25, 0.5, 0.75, 1.0):
        b = SeedBuilder(S, freqs, fs, SeedConfig(strength=s, seed=a.seed))
        write_wav(out / f"melody_s{int(s * 100):03d}.wav", b.melody(notes), b.cfg.sr, 2)
        write_wav(out / f"timbre_s{int(s * 100):03d}.wav", b.native(), b.cfg.sr, 2)
    # reference upper bound: every partial snapped fully to the integer series (no glitch left)
    ref = SeedBuilder(S, freqs, fs, SeedConfig(strength=1.0, seed=a.seed, variation=0.0, ramp=0.0,
                                               p_snap=1.0, p_leave=0.0, drift_cents=0.0))
    write_wav(out / "melody_REF_fully_harmonic.wav", ref.melody(notes), ref.cfg.sr, 2)
    print(f"wrote 11 files to {out}")


def _melody_kw(a):
    return dict(root=a.root, scale=a.scale, bpm=a.bpm, leap_prob=a.leap) if a.style == "motif" else {}


def _bpm(a):
    return a.bpm if a.style == "motif" else 120.0   # classic is fixed at beat = 0.5 s


def notes_to_json(notes):
    return json.dumps([{"start": round(n.start, 4), "dur": round(n.dur, 4), "midi": n.midi,
                        "vel": n.vel} for n in notes], indent=1)


def cmd_melody(a):
    notes = make_melody(a.style, a.minutes, a.seed, **_melody_kw(a))
    pathlib.Path(a.out).write_text(notes_to_json(notes))
    if a.midi_out:
        write_midi(a.midi_out, notes, _bpm(a))
    print(f"wrote {a.out}: {len(notes)} notes, {max(n.start + n.dur for n in notes):.1f}s")


def cmd_long(a):
    S, freqs, fs = drawn_spectrogram(seed=a.seed)
    notes = make_melody(a.style, a.minutes, a.seed, **_melody_kw(a))
    if a.notes_out:
        pathlib.Path(a.notes_out).write_text(notes_to_json(notes))
    if a.midi_out:
        write_midi(a.midi_out, notes, _bpm(a))
    b = SeedBuilder(S, freqs, fs, SeedConfig(strength=a.strength, seed=a.seed))
    y = b.melody(notes)
    write_wav(a.out, y, b.cfg.sr, 1 if a.mono else 2)
    print(f"notes={len(notes)} wrote {a.out} ({len(y) / b.cfg.sr:.1f}s)")


def cmd_tomidi(a):
    notes = load_notes(a.notes)
    write_midi(a.out, notes, a.bpm)
    print(f"wrote {a.out}: {len(notes)} notes at {a.bpm:g} bpm")


def cmd_render(a):
    S, freqs, fs = load_spectrogram(a.input, a.fmax, a.frame_sec)
    gs = load_gs(a.gs, S.shape[1]) if a.gs else None
    cfg = SeedConfig(strength=a.strength, seed=a.seed, variation=a.variation, ramp=a.ramp)
    b = SeedBuilder(S, freqs, fs, cfg, gs_spectrum=gs)
    y = b.melody(load_notes(a.notes)) if a.notes else b.native()
    write_wav(a.out, y, cfg.sr, 2)
    print(f"wrote {a.out} ({len(y) / cfg.sr:.2f}s)")


def main(argv=None):
    ap = argparse.ArgumentParser(prog="idm_seed")
    sp = ap.add_subparsers(dest="cmd", required=True)
    d = sp.add_parser("demo")
    d.add_argument("--out", required=True)
    d.add_argument("--seed", type=int, default=0)
    d.set_defaults(fn=cmd_demo)
    def melody_opts(q):
        q.add_argument("--style", choices=["classic", "motif"], default="classic",
                       help="classic = original (fixed A minor pentatonic, 120 bpm); motif = stepwise, key/scale/bpm options")
        q.add_argument("--root", type=int, default=57, help="motif: tonic as MIDI note (57 = A3)")
        q.add_argument("--scale", choices=sorted(SCALES), default="minor", help="motif: scale")
        q.add_argument("--bpm", type=float, default=120.0, help="motif: tempo")
        q.add_argument("--leap", type=float, default=0.12, help="motif: probability of a leap (0 = fully stepwise)")

    mg = sp.add_parser("melody", help="write only the notes JSON (use with render --notes)")
    mg.add_argument("--minutes", type=float, default=3.0)
    mg.add_argument("--seed", type=int, default=0)
    mg.add_argument("--out", required=True)
    mg.add_argument("--midi-out", help="also write a .mid file")
    melody_opts(mg)
    mg.set_defaults(fn=cmd_melody)
    lg = sp.add_parser("long", help="full-length melody seed (e.g. --minutes 10)")
    melody_opts(lg)
    lg.add_argument("--notes-out", help="also save the notes as JSON")
    lg.add_argument("--midi-out", help="also write a .mid file")
    lg.add_argument("--minutes", type=float, default=10.0)
    lg.add_argument("--strength", type=float, default=0.5)
    lg.add_argument("--seed", type=int, default=0)
    lg.add_argument("--mono", action="store_true", help="half the file size (stereo is only a duplicate)")
    lg.add_argument("--out", required=True)
    lg.set_defaults(fn=cmd_long)
    tm = sp.add_parser("tomidi", help="convert a notes JSON to a .mid file")
    tm.add_argument("--notes", required=True)
    tm.add_argument("--bpm", type=float, default=120.0, help="tempo written into the file (grid only)")
    tm.add_argument("--out", required=True)
    tm.set_defaults(fn=cmd_tomidi)
    r = sp.add_parser("render")
    r.add_argument("--input", required=True)
    r.add_argument("--notes")
    r.add_argument("--gs", help=".npy mean spectrum of GS sound-source renders (same bins or resampled)")
    r.add_argument("--out", required=True)
    r.add_argument("--strength", type=float, default=0.5)
    r.add_argument("--variation", type=float, default=0.25)
    r.add_argument("--ramp", type=float, default=0.5)
    r.add_argument("--seed", type=int, default=0)
    r.add_argument("--fmax", type=float, default=8000.0)
    r.add_argument("--frame-sec", type=float, default=0.032)
    r.set_defaults(fn=cmd_render)
    a = ap.parse_args(argv)
    a.fn(a)


if __name__ == "__main__":
    main()
