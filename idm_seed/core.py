"""idm_seed.core: harmonic detection and *partial* (never complete) harmonic correction.

Spectrogram S: [T, F] non-negative linear magnitude. freqs: [F] bin centre Hz, increasing.

Unlike a pitch tracker that assumes near-integer partials, a drawn spectrogram may place
partials anywhere. Every local peak is assigned to the nearest integer multiple of f0
(k = round(hz / f0)); the strongest peak per k becomes "partial k". Anything else stays
in the residual and is left untouched (that is part of the glitch).
"""
from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np


# ------------------------------------------------------------------ peak helpers
def index_to_hz(freqs, idx):
    return np.interp(idx, np.arange(len(freqs), dtype=float), freqs)


def parabolic_peak_hz(frame, freqs, j):
    if j <= 0 or j >= len(frame) - 1:
        return float(freqs[j])
    a, b, c = np.log(frame[j - 1 : j + 2] + 1e-12)
    den = a - 2 * b + c
    off = 0.0 if den == 0 else float(np.clip(0.5 * (a - c) / den, -0.5, 0.5))
    return float(index_to_hz(freqs, j + off))


def local_peaks(frame, floor):
    if len(frame) < 3:
        return np.empty(0, dtype=int)
    m = (frame[1:-1] > frame[:-2]) & (frame[1:-1] >= frame[2:]) & (frame[1:-1] >= floor)
    return np.nonzero(m)[0] + 1


def find_f0_lowest(frame, freqs, f0_min, f0_max, rel):
    """f0 = lowest significant peak in [f0_min, f0_max]. NaN if none (unvoiced)."""
    mx = frame.max()
    if mx <= 0:
        return float("nan")
    idx = local_peaks(frame, rel * mx)
    idx = idx[(freqs[idx] >= f0_min) & (freqs[idx] <= f0_max)]
    if idx.size == 0:
        return float("nan")
    return parabolic_peak_hz(frame, freqs, int(idx[0]))


def find_partials(frame, freqs, f0, n_harm, floor_rel):
    """dict k -> (hz, amp); k=1 is f0 itself. Strongest peak wins each integer slot."""
    mx = frame.max()
    best = {1: (f0, float(np.interp(f0, freqs, frame)))}
    for j in local_peaks(frame, floor_rel * mx):
        hz = parabolic_peak_hz(frame, freqs, int(j))
        k = int(round(hz / f0))
        if k < 2 or k > n_harm:
            continue
        a = float(frame[j])
        if k not in best or a > best[k][1]:
            best[k] = (hz, a)
    return best


# --------------------------------------------------- step 1: harmonic structure
@dataclass
class HarmonicStructure:
    f0: np.ndarray            # [T] Hz, NaN where unvoiced
    confidence: np.ndarray    # [T]
    partial_hz: np.ndarray    # [T, K] NaN if the slot is empty
    partial_amp: np.ndarray   # [T, K]
    harmonic_mask: np.ndarray # [T, F] int16, 0 = residual, k = lobe of partial k


def detect_harmonic_structure(S, freqs, f0_min=60.0, f0_max=1000.0, n_harm=10,
                              f0_rel=0.25, partial_rel=0.03):
    T, F = S.shape
    f0 = np.full(T, np.nan)
    conf = np.zeros(T)
    phz = np.full((T, n_harm), np.nan)
    pamp = np.zeros((T, n_harm))
    mask = np.zeros((T, F), dtype=np.int16)
    for t in range(T):
        fr = S[t]
        f = find_f0_lowest(fr, freqs, f0_min, f0_max, f0_rel)
        if np.isnan(f):
            continue
        f0[t], conf[t] = f, 1.0
        best = find_partials(fr, freqs, f, n_harm, partial_rel)
        pk = [(k, best[k][0]) for k in sorted(best)]
        for k, (h, a) in best.items():
            phz[t, k - 1], pamp[t, k - 1] = h, a
        for n, (k, p) in enumerate(pk):
            lo = p - 0.5 * f if n == 0 else 0.5 * (pk[n - 1][1] + p)
            hi = p + 0.5 * f if n == len(pk) - 1 else 0.5 * (p + pk[n + 1][1])
            mask[t, (freqs >= lo) & (freqs < hi)] = k
    return HarmonicStructure(f0, conf, phz, pamp, mask)


# ------------------------------------------------------ step 2: partial correction
@dataclass
class GlitchPlan:
    """Seeded and independent of strength, so a sweep over s is coherent."""
    gain: np.ndarray         # [K] g_k in [0,1]; 0 = this partial is never corrected
    drift_cents: np.ndarray  # [K] extra intentional detune for left-alone partials


def make_glitch_plan(n_harm, seed, p_snap=0.3, p_leave=0.25, drift_cents=0.0):
    rng = np.random.default_rng(seed)
    u = rng.random(n_harm)
    g = rng.uniform(0.4, 1.0, n_harm)
    g[u < p_snap] = 1.0
    leave = u >= 1.0 - p_leave
    g[leave] = 0.0
    g[0] = 1.0
    drift = np.where(leave, rng.normal(0.0, 1.0, n_harm) * drift_cents, 0.0)
    return GlitchPlan(g, drift)


def apply_partial_correction(S, freqs, hs: HarmonicStructure, strength, plan: GlitchPlan):
    """strength: scalar or [T] array. Partial k at f_k moves to f_k + s*g_k*(k*f0 - f_k)
    (+ drift for left-alone partials). Residual bins are untouched. Lobe energy preserved."""
    T, F = S.shape
    s_t = np.broadcast_to(np.asarray(strength, dtype=float), (T,))
    out = S.copy()
    for t in range(T):
        if np.isnan(hs.f0[t]):
            continue
        frame, acc, removed = S[t], np.zeros(F), np.zeros(F, dtype=bool)
        for k in range(1, hs.partial_hz.shape[1] + 1):
            fk = hs.partial_hz[t, k - 1]
            if np.isnan(fk):
                continue
            sel = hs.harmonic_mask[t] == k
            if not sel.any():
                continue
            delta = s_t[t] * plan.gain[k - 1] * (k * hs.f0[t] - fk)
            if plan.drift_cents[k - 1] != 0.0:
                delta += s_t[t] * fk * (2 ** (plan.drift_cents[k - 1] / 1200) - 1)
            idx = np.nonzero(sel)[0]
            shifted = np.interp(freqs - delta, freqs[idx], frame[idx], left=0.0, right=0.0)
            e1 = np.sum(shifted ** 2)
            if e1 > 0:
                shifted *= np.sqrt(np.sum(frame[idx] ** 2) / e1)
            acc += shifted ** 2
            removed |= sel
        out[t] = np.sqrt(np.where(removed, 0.0, frame) ** 2 + acc)
    return out


# --------------------------------------------------------- step 3: novelty filter
def cosine_similarity(a, b):
    a, b = np.asarray(a, float), np.asarray(b, float)
    return float(a @ b / (np.linalg.norm(a) * np.linalg.norm(b) + 1e-12))
