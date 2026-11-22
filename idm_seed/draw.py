"""Synthetic stand-in for a hand-drawn spectrogram: physically impossible partial ratios,
wobble, dropouts and stray blobs. Used by the demo and the tests."""
from __future__ import annotations

import numpy as np


def drawn_spectrogram(seed=0, frames=300, sr=16000, n_fft=2048, hop=512, f0=440.0,
                      n_partials=9, max_offset=0.45, sigma_hz=12.0, wobble=0.04,
                      dropout=0.04, blobs=6, tilt=0.8, min_offset=0.35):
    """Returns S [T,F], freqs [F], frame_sec. Partial k sits at (k + offset_k) * f0 with
    |offset_k| in [min_offset, max_offset] (k>=2): inharmonic by up to ~half an f0 per partial."""
    rng = np.random.default_rng(seed)
    freqs = np.fft.rfftfreq(n_fft, 1.0 / sr)
    S = np.zeros((frames, len(freqs)))
    # every partial is clearly off the integer series (|offset| >= min_offset), whatever the seed
    offs = rng.choice([-1.0, 1.0], n_partials) * rng.uniform(min_offset, max_offset, n_partials)
    offs[0] = 0.0
    amps = (1.0 / np.arange(1, n_partials + 1) ** tilt) * rng.uniform(0.6, 1.0, n_partials)
    walk = np.cumsum(rng.normal(0, wobble / 10, (frames, n_partials)), axis=0)
    walk = np.clip(walk, -wobble, wobble)
    walk[:, 0] = 0.0
    for k in range(n_partials):
        centre = (k + 1 + offs[k] + walk[:, k]) * f0
        live = rng.random(frames) > dropout
        live[0] = True
        a = amps[k] * live
        keep = centre < sr / 2 - 4 * sigma_hz
        d = freqs[None, :] - centre[:, None]
        S += np.where(keep[:, None], a[:, None] * np.exp(-0.5 * (d / sigma_hz) ** 2), 0.0)
    for _ in range(blobs):  # stray non-harmonic smudges = residual glitch
        t0, w = rng.integers(0, frames), rng.integers(5, 30)
        fc = rng.uniform(1.5 * f0, 8.5 * f0)
        d = freqs - fc
        S[t0:t0 + w] += 0.12 * np.exp(-0.5 * (d / (2 * sigma_hz)) ** 2)[None, :]
    S += 1e-4 * rng.random(S.shape)
    return S, freqs, hop / sr
