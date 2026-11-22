"""CLI.  python -m idm_seed demo --out DIR
        python -m idm_seed render --input art.png|spec.npy [--notes notes.json] --strength 0.5 --out seed.wav
"""
from __future__ import annotations

import argparse
import json
import pathlib

import numpy as np

from .draw import drawn_spectrogram
from . import library as lib
from .inputs import load_gs, load_notes, load_spectrogram, notes_to_json  # noqa: F401
from .metrics import analyze_wav
from .midi import write_midi
from .melody import SCALES, make_classic_melody as make_long_melody, make_melody  # noqa: F401
from .batch import parse_grid, run_batch
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


def cmd_batch(a):
    if a.recipe:
        base = json.loads(pathlib.Path(a.recipe).read_text(encoding="utf-8"))
    else:
        base = {"input": {"kind": "drawn", "seed": a.input_seed},
                "melody": {"kind": "generator", "style": a.style, "minutes": a.minutes, "seed": a.seed,
                           "options": _melody_kw(a)},
                "config": default_config(strength=a.strength, seed=a.seed), "gs": None, "channels": 1 if a.mono else 2}
    weights = json.loads(pathlib.Path(a.weights).read_text(encoding="utf-8")) if a.weights else None
    rows = run_batch(base, parse_grid(a.grid), a.out, keep=a.keep,
                     group_by=a.group_by, weights=weights, max_candidates=a.max_candidates)
    print(f"{len(rows)} candidates scored -> {pathlib.Path(a.out) / 'ranking.csv'}")
    for r in rows:
        if r["wav"]:
            m = r["metrics"]
            print(f"  [{r['group']}] #{r['rank']} score={r['score']:.3f} periodicity={m['periodicity']:.2f} "
                  f"roughness={m['roughness']:.4f}  {r['wav']}")


def cmd_library(a):
    if a.lib_cmd == "add":
        files = sorted(pathlib.Path(a.dir).glob("*.json")) if a.dir else [pathlib.Path(a.sidecar)]
        n = 0
        for f in files:
            if f.name in ("batch.json",):
                continue
            rec = lib.add_seed(a.library, f, tags=a.tags.split(",") if a.tags else ())
            n += 1
            print(f"added {rec['id']}  {rec['wav']}")
        print(f"{n} seed(s) -> {a.library}")
    elif a.lib_cmd == "note":
        rec = lib.add_note(a.library, a.id, rating=a.rating, engine=a.engine, mode=a.mode,
                           text=a.text or "", tags=a.tags.split(",") if a.tags else ())
        print(f"noted {rec['id']}: rating={rec['rating']} {rec['engine']}/{rec['mode']} {rec['text'][:60]}")
    elif a.lib_cmd == "list":
        rows, broken = lib.table(a.library)
        if a.min_rating is not None:
            rows = [r for r in rows if r["rating"] is not None and r["rating"] >= a.min_rating]
        key = {"rating": lambda r: -(r["rating"] if r["rating"] is not None else -1),
               "periodicity": lambda r: -((r.get("metrics") or {}).get("periodicity", -1)),
               "added": lambda r: r["added_utc"]}[a.sort]
        for r in sorted(rows, key=key):
            m = r.get("metrics") or {}
            note = (r["last_note"] or {}).get("text", "")[:40]
            rating = "-" if r["rating"] is None else "%.1f" % r["rating"]
            per = m.get("periodicity", float("nan"))
            print("%s  rating=%s  s=%s  per=%.2f  %s  %s"
                  % (r["id"], rating, r["summary"].get("strength"), per, r["wav"], note))
        if broken:
            print(f"({broken} unreadable line(s) ignored)")
    elif a.lib_cmd == "stats":
        print(json.dumps(lib.stats(a.library), indent=1, ensure_ascii=False))


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
    bt = sp.add_parser("batch", help="render a grid of candidates, score them, keep the best per group")
    melody_opts(bt)
    bt.add_argument("--recipe", help="base recipe JSON (overrides the options below)")
    bt.add_argument("--grid", action="append", metavar="PATH=V1,V2,...",
                    help="e.g. --grid config.seed=0,1,2,3 --grid config.strength=0.3,0.5,0.7 (repeatable)")
    bt.add_argument("--keep", type=int, default=3, help="candidates to keep (WAV+sidecar) per group")
    bt.add_argument("--group-by", default="config.strength", help="rank inside groups of this grid path")
    bt.add_argument("--weights", help="JSON with score weights (see batch.DEFAULT_WEIGHTS)")
    bt.add_argument("--max-candidates", type=int, default=200)
    bt.add_argument("--minutes", type=float, default=2.0)
    bt.add_argument("--strength", type=float, default=0.5)
    bt.add_argument("--seed", type=int, default=0, help="melody/config seed when not in the grid")
    bt.add_argument("--input-seed", type=int, default=0, help="seed of the synthetic drawing")
    bt.add_argument("--mono", action="store_true")
    bt.add_argument("--out", required=True)
    bt.set_defaults(fn=cmd_batch)
    lb = sp.add_parser("library", help="append-only seed library with your notes on how seeds did in Suno")
    lb.add_argument("--library", default="library.jsonl")
    ls = lb.add_subparsers(dest="lib_cmd", required=True)
    la = ls.add_parser("add", help="register seed(s) from sidecar json file(s)")
    g = la.add_mutually_exclusive_group(required=True)
    g.add_argument("--sidecar"); g.add_argument("--dir", help="register every sidecar in a folder (e.g. a batch output)")
    la.add_argument("--tags")
    ln = ls.add_parser("note", help="record how a seed did (id or unique prefix of 4+ chars)")
    ln.add_argument("--id", required=True)
    ln.add_argument("--rating", type=int, help="1-5")
    ln.add_argument("--engine", default="suno")
    ln.add_argument("--mode", help="cover / extend / ...")
    ln.add_argument("--text"); ln.add_argument("--tags")
    ll = ls.add_parser("list")
    ll.add_argument("--sort", choices=["rating", "periodicity", "added"], default="rating")
    ll.add_argument("--min-rating", type=float)
    ls.add_parser("stats", help="rank correlation between metrics and your ratings + suggested score weights")
    lb.set_defaults(fn=cmd_library)
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
