"""Recipes and sidecars: everything needed to rebuild a seed, and a check that it still rebuilds
bit-for-bit.

recipe = {
  "input":  {"kind": "drawn", "seed": 0, "params": {...}}
          | {"kind": "file", "path": "art.png", "sha256": "...", "fmax": 8000, "frame_sec": 0.032},
  "melody": {"kind": "generator", "style": "motif", "minutes": 10, "seed": 0, "options": {...}}
          | {"kind": "notes", "path": "notes.json", "sha256": "..."}
          | {"kind": "native"},                      # no notes: follow the drawing's own pitch
  "config": {SeedConfig fields},
  "gs":     null | {"path": "gs.npy", "sha256": "..."},
  "channels": 1 | 2
}
A sidecar sits next to the WAV (seed.wav -> seed.json) and holds the recipe, the hash of the PCM
data that was written, the objective metrics and the environment. Rebuilding on another machine
or numpy version may differ in the last bits: verify() then reports it instead of pretending.
"""
from __future__ import annotations

import dataclasses
import datetime
import hashlib
import json
import pathlib
import platform
import wave

import numpy as np

from .draw import drawn_spectrogram
from .inputs import load_gs, load_notes, load_spectrogram
from .melody import make_melody
from .metrics import analyze
from .pipeline import SeedBuilder, SeedConfig
from .render import to_pcm, write_wav

SCHEMA = 1


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def recipe_id(recipe):
    """Short stable id of what the recipe says (not of the output)."""
    return hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest()[:12]


def _resolve(path, base_dir):
    p = pathlib.Path(path)
    if p.is_absolute() or base_dir is None:
        return p
    return pathlib.Path(base_dir) / p


def _checked(path, expect, base_dir, what):
    p = _resolve(path, base_dir)
    if not p.exists():
        raise FileNotFoundError(f"{what} not found: {p}")
    if expect and sha256_file(p) != expect:
        raise ValueError(f"{what} changed since the recipe was made: {p}")
    return p


def build_seed(recipe, base_dir=None):
    """recipe -> (y float array, notes or None). Deterministic for a given recipe and inputs."""
    inp = recipe["input"]
    if inp["kind"] == "drawn":
        S, freqs, fs = drawn_spectrogram(seed=inp["seed"], **inp.get("params", {}))
    elif inp["kind"] == "file":
        p = _checked(inp["path"], inp.get("sha256"), base_dir, "input spectrogram")
        S, freqs, fs = load_spectrogram(p, inp.get("fmax", 8000.0), inp.get("frame_sec", 0.032))
    else:
        raise ValueError(f"unknown input kind: {inp['kind']}")
    gs = None
    if recipe.get("gs"):
        g = recipe["gs"]
        gs = load_gs(_checked(g["path"], g.get("sha256"), base_dir, "gs spectrum"), S.shape[1])
    mel = recipe["melody"]
    if mel["kind"] == "generator":
        notes = make_melody(mel["style"], mel["minutes"], mel["seed"], **mel.get("options", {}))
    elif mel["kind"] == "notes":
        notes = load_notes(_checked(mel["path"], mel.get("sha256"), base_dir, "notes"))
    elif mel["kind"] == "native":
        notes = None
    else:
        raise ValueError(f"unknown melody kind: {mel['kind']}")
    b = SeedBuilder(S, freqs, fs, SeedConfig(**recipe["config"]), gs_spectrum=gs)
    return (b.melody(notes) if notes else b.native()), notes


def default_config(**over):
    return {**dataclasses.asdict(SeedConfig()), **over}


def file_ref(path, base_dir=None):
    """{'path': posix path relative to base_dir when possible, 'sha256': ...} for recipe inputs."""
    p = pathlib.Path(path)
    shown = p
    if base_dir is not None:
        try:
            shown = p.resolve().relative_to(pathlib.Path(base_dir).resolve())
        except ValueError:
            shown = p.resolve()
    return {"path": shown.as_posix(), "sha256": sha256_file(p)}


def sidecar_path(wav_path):
    return pathlib.Path(wav_path).with_suffix(".json")


def write_seed(wav_path, recipe, with_metrics=True):
    """Build, write WAV + sidecar JSON. Relative input paths in the recipe resolve against the
    WAV's folder. Returns the sidecar dict."""
    wav_path = pathlib.Path(wav_path)
    base = wav_path.parent
    y, notes = build_seed(recipe, base)
    ch = recipe.get("channels", 2)
    sr = recipe["config"]["sr"]
    write_wav(wav_path, y, sr, ch)
    metrics = None
    if with_metrics and notes:
        try:
            metrics = analyze(y, sr, notes)
        except ValueError as e:        # e.g. no note long enough to measure
            metrics = {"error": str(e)}
    side = {
        "schema": SCHEMA, "tool": "idm_seed", "recipe_id": recipe_id(recipe), "recipe": recipe,
        "output": {"wav": wav_path.name, "pcm_sha256": hashlib.sha256(to_pcm(y, ch).tobytes()).hexdigest(),
                   "sample_rate": sr, "channels": ch, "seconds": round(len(y) / sr, 3),
                   "notes": len(notes) if notes else None},
        "metrics": metrics,
        "environment": {"python": platform.python_version(), "numpy": np.__version__,
                        "platform": platform.platform()},
        "created_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
    }
    sidecar_path(wav_path).write_text(json.dumps(side, indent=1, ensure_ascii=False), encoding="utf-8")
    return side


def read_sidecar(path):
    side = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    if side.get("schema") != SCHEMA:
        raise ValueError(f"unsupported sidecar schema: {side.get('schema')}")
    return side


def verify(sidecar_file):
    """Is the WAV on disk the one the sidecar describes, and does the recipe still rebuild it
    bit-for-bit? Returns a dict of booleans plus the two hashes."""
    sidecar_file = pathlib.Path(sidecar_file)
    side = read_sidecar(sidecar_file)
    out = side["output"]
    ch = out["channels"]
    res = {"file_matches_sidecar": None, "rebuild_matches_sidecar": None,
           "expected": out["pcm_sha256"], "rebuilt": None}
    wav = sidecar_file.parent / out["wav"]
    if wav.exists():
        with wave.open(str(wav)) as w:
            data = w.readframes(w.getnframes())
        res["file_matches_sidecar"] = hashlib.sha256(data).hexdigest() == out["pcm_sha256"]
    y, _ = build_seed(side["recipe"], sidecar_file.parent)
    res["rebuilt"] = hashlib.sha256(to_pcm(y, ch).tobytes()).hexdigest()
    res["rebuild_matches_sidecar"] = res["rebuilt"] == out["pcm_sha256"]
    res["same_numpy"] = side["environment"]["numpy"] == np.__version__
    return res
