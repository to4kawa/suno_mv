"""Readers/writers for spectrogram, notes and reference-spectrum files."""
from __future__ import annotations

import json
import pathlib

import numpy as np

from .render import Note


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


def notes_to_json(notes):
    return json.dumps([{"start": round(n.start, 4), "dur": round(n.dur, 4), "midi": n.midi,
                        "vel": n.vel} for n in notes], indent=1)
