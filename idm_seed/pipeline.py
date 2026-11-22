"""Glue: drawn spectrogram + melody -> WAV. Every note gets its own glitch plan, its own
strength and its own intermediate-state ramp, so notes do not all share one quirk."""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .core import (apply_partial_correction, cosine_similarity, detect_harmonic_structure,
                   make_glitch_plan)
from .render import (Note, extract_timbre, normalize_peak, render_melody, render_native)


@dataclass
class SeedConfig:
    sr: int = 44100
    n_harm: int = 10
    strength: float = 0.5        # centre of the correction strength; 0 = leave, 1 = full snap
    variation: float = 0.25      # per-note +- jitter of strength
    ramp: float = 0.5            # per-note morph width (sign random): messy->tidy or reverse
    seed: int = 0
    p_snap: float = 0.3
    p_leave: float = 0.25
    drift_cents: float = 60.0    # extra detune on partials that are left alone
    time_mode: str = "native"    # per-note window of the drawing, played at real speed
    tail: float = 0.5
    peak_dbfs: float = -1.0
    # novelty filter (optional): reject when cos_sim(mean spectrum, gs) >= max_similarity
    max_similarity: float = 0.95
    max_retries: int = 4


class SeedBuilder:
    def __init__(self, S, freqs, frame_sec, cfg: SeedConfig, gs_spectrum=None,
                 f0_min=60.0, f0_max=1000.0):
        self.S, self.freqs, self.frame_sec, self.cfg, self.gs = S, freqs, frame_sec, cfg, gs_spectrum
        self.hs = detect_harmonic_structure(S, freqs, f0_min, f0_max, cfg.n_harm)
        if np.all(np.isnan(self.hs.f0)):
            raise ValueError("no voiced frame found in the input spectrogram")
        self.novelty_log = []

    def _timbre(self, key, S, hs, s_arr):
        c = self.cfg
        for attempt in range(c.max_retries + 1):
            plan = make_glitch_plan(c.n_harm, seed=(c.seed, key, attempt), p_snap=c.p_snap,
                                    p_leave=c.p_leave,
                                    drift_cents=c.drift_cents * (1 + attempt))
            out = apply_partial_correction(S, self.freqs, hs, s_arr, plan)
            if self.gs is None:
                return extract_timbre(out, self.freqs, hs.f0, c.n_harm, self.frame_sec)
            sim = cosine_similarity(out.mean(axis=0), self.gs)
            if sim < c.max_similarity:
                self.novelty_log.append((key, attempt + 1, sim, True))
                return extract_timbre(out, self.freqs, hs.f0, c.n_harm, self.frame_sec)
        self.novelty_log.append((key, c.max_retries + 1, sim, False))  # exhausted: keep last
        return extract_timbre(out, self.freqs, hs.f0, c.n_harm, self.frame_sec)

    def _window(self, i, note):
        """Frames of the drawing this note uses. Each note takes a different stretch, so
        notes differ; only that stretch is corrected (cheap even for 10-minute seeds)."""
        T = len(self.S)
        if self.cfg.time_mode == "stretch":
            return 0, T
        n = min(T, int(np.ceil((note.dur + 0.08) / self.frame_sec)) + 2)
        rng = np.random.default_rng((self.cfg.seed, i, 11))
        for _ in range(32):
            a = int(rng.integers(0, T - n + 1))
            if np.any(~np.isnan(self.hs.f0[a:a + n])):
                return a, a + n
        a = min(int(np.nonzero(~np.isnan(self.hs.f0))[0][0]), T - n)
        return a, a + n

    def note_strength_curve(self, i, n):
        """Strength over the note's n frames: jittered centre, random-direction morph."""
        c = self.cfg
        rng = np.random.default_rng((c.seed, i, 7))
        s_i = np.clip(c.strength + rng.uniform(-c.variation, c.variation), 0, 1)
        w = c.ramp * rng.choice([-1.0, 1.0])
        return np.clip(s_i + w * np.linspace(-0.5, 0.5, n), 0, 1)

    def melody(self, notes):
        c = self.cfg
        cache = {}

        def timbre_for_note(i, n):
            if i not in cache:
                a, b = self._window(i, n)
                cache[i] = self._timbre(i, self.S[a:b], self.hs.slice(a, b),
                                        self.note_strength_curve(i, b - a))
            return cache[i]

        y = render_melody(notes, timbre_for_note, sr=c.sr, tail=c.tail, time_mode=c.time_mode)
        return normalize_peak(y, c.peak_dbfs)

    def native(self):
        """Mode A: no notes; follow the drawing's own f0 contour. Length from the input."""
        c = self.cfg
        s = np.clip(c.strength + c.ramp * np.linspace(-0.5, 0.5, len(self.S)), 0, 1)
        tim = self._timbre(1_000_000, self.S, self.hs, s)  # non-negative int key: RNG seed part
        return normalize_peak(render_native(tim, self.hs.f0, sr=c.sr, tail=c.tail), c.peak_dbfs)
