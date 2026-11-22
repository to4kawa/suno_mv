"""Melody makers. Both return a list of Note ordered by start time; length is exact to the last
note that still fits inside `minutes`.

classic : the original generator (A minor pentatonic, 120 bpm, random picks). Kept unchanged
          because seeds made with it were already used on Suno.
motif   : stepwise motion with occasional leaps, a 2-bar motif + answer per 8-beat phrase,
          cadence on tonic/fifth, form A A B A C C B A (repeats give Cover a skeleton).
          Key, scale and tempo are parameters.
"""
from __future__ import annotations

import numpy as np

from .render import Note

PENTA = [57, 60, 62, 64, 67, 69, 72]  # classic: A minor pentatonic-ish, A3..C5

SCALES = {
    "minor_pentatonic": [0, 3, 5, 7, 10],
    "major_pentatonic": [0, 2, 4, 7, 9],
    "minor": [0, 2, 3, 5, 7, 8, 10],
    "major": [0, 2, 4, 5, 7, 9, 11],
    "dorian": [0, 2, 3, 5, 7, 9, 10],
    "phrygian": [0, 1, 3, 5, 7, 8, 10],
    "whole_tone": [0, 2, 4, 6, 8, 10],
}

_RHYTHMS = [[1, 1, 1, 1], [1.5, .5, 1, 1], [.5, .5, 1, 2], [1, .5, .5, 2], [2, 1, 1],
            [.5, .5, .5, .5, 1, 1], [1, 1, 2], [1, 2, 1], [.5, .5, 1, .5, .5, 1]]


def make_classic_melody(minutes, seed=0, beat=0.5):
    """Original generator, behaviour unchanged."""
    rng = np.random.default_rng(seed)

    def phrase():
        pos, out = 0, []
        while pos < 8:
            d = float(rng.choice([0.5, 1.0, 1.0, 2.0]))
            d = min(d, 8 - pos)
            out.append((pos, d, int(PENTA[int(rng.integers(0, len(PENTA)))]) if rng.random() > 0.12 else None))
            pos += d
        return out

    A, B, C = phrase(), phrase(), phrase()
    form = [A, A, B, A, C, C, B, A]
    notes, t, limit = [], 0.0, minutes * 60.0
    i = 0
    while True:
        for pos, d, m in form[i % len(form)]:
            s, dur = t + pos * beat, d * beat * 0.92
            if s + dur > limit:
                return notes
            if m is not None:  # None = rest
                notes.append(Note(s, dur, m, 0.9))
        t += 8 * beat
        i += 1


def _lattice(root, scale, span_semitones):
    steps = SCALES[scale]
    return sorted(root + o * 12 + s for o in range(0, 4) for s in steps if s + o * 12 <= span_semitones)


def make_motif_melody(minutes, seed=0, root=57, scale="minor", bpm=120.0, leap_prob=0.12,
                      rest_prob=0.08, span=19, vel=0.9):
    """root: MIDI note of the tonic. span: semitones above the root that may be used."""
    rng = np.random.default_rng(seed)
    pitches = _lattice(root, scale, span)
    nd = len(pitches)
    fifth = next((i for i, p in enumerate(pitches) if p - root == 7), nd // 2)
    cadence = [0, fifth, 0]

    def walk(deg, n):
        out = []
        for _ in range(n):
            if rng.random() < leap_prob:
                deg += int(rng.choice([-4, -3, 3, 4]))
            else:
                deg += int(rng.choice([-2, -1, 0, 1, 2], p=[.15, .3, .1, .3, .15]))
            if deg < 0:
                deg = -deg
            if deg > nd - 1:
                deg = 2 * (nd - 1) - deg
            deg = int(np.clip(deg, 0, nd - 1))
            out.append(deg)
        return out

    def phrase(start):
        r1 = _RHYTHMS[int(rng.integers(len(_RHYTHMS)))]
        r2 = _RHYTHMS[int(rng.integers(len(_RHYTHMS)))]
        d1 = walk(start, len(r1))
        d2 = walk(d1[-1], len(r2))
        d2[-1] = int(rng.choice(cadence))            # answer ends on tonic / fifth
        out, pos = [], 0.0
        for d, r in zip(d1 + d2, r1 + r2):
            out.append((pos, float(r), None if rng.random() < rest_prob else d))
            pos += r
        return out

    A, B, C = phrase(0), phrase(2), phrase(4)
    form = [A, A, B, A, C, C, B, A]
    beat = 60.0 / bpm
    notes, t, limit, i = [], 0.0, minutes * 60.0, 0
    while True:
        for pos, d, deg in form[i % len(form)]:
            s, dur = t + pos * beat, d * beat * 0.92
            if s + dur > limit:
                return notes
            if deg is not None:
                notes.append(Note(s, dur, float(pitches[deg]), vel))
        t += 8 * beat
        i += 1


def make_melody(style, minutes, seed=0, **kw):
    if style == "classic":
        return make_classic_melody(minutes, seed)
    if style == "motif":
        return make_motif_melody(minutes, seed, **kw)
    raise ValueError(f"unknown melody style: {style}")
