"""CLI.  python -m idm_seed demo --out DIR
        python -m idm_seed render --input art.png|spec.npy [--notes notes.json] --strength 0.5 --out seed.wav
"""
from __future__ import annotations

import argparse
import json
import pathlib

import numpy as np

from .draw import drawn_spectrogram
from .inputs import load_gs, load_notes, load_spectrogram, notes_to_json  # noqa: F401
from .metrics import analyze_wav
from .midi import write_midi
from .melody import SCALES, make_classic_melody as make_long_melody, make_melody  # noqa: F401
from .pipeline import SeedBuilder, SeedConfig
from .recipe import default_config, file_ref, verify, write_seed
from .render import Note, write_wav


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


def cmd_melody(a):
    notes = make_melody(a.style, a.minutes, a.seed, **_melody_kw(a))
    pathlib.Path(a.out).write_text(notes_to_json(notes))
    if a.midi_out:
        write_midi(a.midi_out, notes, _bpm(a))
    print(f"wrote {a.out}: {len(notes)} notes, {max(n.start + n.dur for n in notes):.1f}s")


def cmd_long(a):
    recipe = {"input": {"kind": "drawn", "seed": a.seed},
              "melody": {"kind": "generator", "style": a.style, "minutes": a.minutes, "seed": a.seed,
                         "options": _melody_kw(a)},
              "config": default_config(strength=a.strength, seed=a.seed),
              "gs": None, "channels": 1 if a.mono else 2}
    if a.notes_out or a.midi_out:
        notes = make_melody(a.style, a.minutes, a.seed, **_melody_kw(a))
        if a.notes_out:
            pathlib.Path(a.notes_out).write_text(notes_to_json(notes))
        if a.midi_out:
            write_midi(a.midi_out, notes, _bpm(a))
    side = write_seed(a.out, recipe)
    o = side["output"]
    print(f"notes={o['notes']} wrote {a.out} ({o['seconds']:.1f}s) + {pathlib.Path(a.out).with_suffix('.json').name}")


def cmd_tomidi(a):
    notes = load_notes(a.notes)
    write_midi(a.out, notes, a.bpm)
    print(f"wrote {a.out}: {len(notes)} notes at {a.bpm:g} bpm")


def cmd_analyze(a):
    res = analyze_wav(a.wav, load_notes(a.notes), max_notes=a.max_notes)
    print(json.dumps(res, indent=1))


def cmd_render(a):
    base = pathlib.Path(a.out).resolve().parent
    recipe = {"input": {"kind": "file", **file_ref(a.input, base), "fmax": a.fmax, "frame_sec": a.frame_sec},
              "melody": ({"kind": "notes", **file_ref(a.notes, base)} if a.notes else {"kind": "native"}),
              "config": default_config(strength=a.strength, seed=a.seed, variation=a.variation, ramp=a.ramp),
              "gs": ({**file_ref(a.gs, base)} if a.gs else None), "channels": 2}
    side = write_seed(a.out, recipe)
    print(f"wrote {a.out} ({side['output']['seconds']:.2f}s) + {pathlib.Path(a.out).with_suffix('.json').name}")


def cmd_verify(a):
    res = verify(a.sidecar)
    print(json.dumps(res, indent=1))
    raise SystemExit(0 if res["rebuild_matches_sidecar"] and res["file_matches_sidecar"] in (True, None) else 1)


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
    an = sp.add_parser("analyze", help="objective metrics of a seed WAV, given its notes JSON")
    an.add_argument("--wav", required=True)
    an.add_argument("--notes", required=True)
    an.add_argument("--max-notes", type=int, default=64)
    an.set_defaults(fn=cmd_analyze)
    vf = sp.add_parser("verify", help="check a seed.json: WAV unchanged, and the recipe rebuilds it bit-for-bit")
    vf.add_argument("--sidecar", required=True)
    vf.set_defaults(fn=cmd_verify)
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
