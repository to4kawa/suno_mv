"""Additive synthesis from partial tracks, melody scheduling and WAV export (numpy + wave)."""
from __future__ import annotations

import wave
from dataclasses import dataclass

import numpy as np

from .core import find_partials


@dataclass
class Timbre:
    """Pitch-independent: partial k at ratio[t,k]*f0 with amplitude amp[t,k]."""
    frame_sec: float
    ratio: np.ndarray  # [T, K] (NaN-free after filling)
    amp: np.ndarray    # [T, K] normalised, max == 1


@dataclass
class Note:
    start: float
    dur: float
    midi: float
    vel: float = 1.0


def midi_to_hz(m):
    return 440.0 * 2 ** ((np.asarray(m, float) - 69) / 12)


def extract_timbre(S_corr, freqs, f0, n_harm, frame_sec, partial_rel=0.03):
    T = S_corr.shape[0]
    ratio = np.full((T, n_harm), np.nan)
    amp = np.zeros((T, n_harm))
    for t in range(T):
        if np.isnan(f0[t]):
            continue
        for k, (hz, a) in find_partials(S_corr[t], freqs, f0[t], n_harm, partial_rel).items():
            ratio[t, k - 1], amp[t, k - 1] = hz / f0[t], a
    if not np.any(~np.isnan(ratio[:, 0])):
        raise ValueError("no voiced frame found: nothing to build a timbre from")
    amp /= max(amp.max(), 1e-12)
    for k in range(n_harm):
        good = ~np.isnan(ratio[:, k])
        if good.any():
            ratio[~good, k] = np.interp(np.nonzero(~good)[0], np.nonzero(good)[0], ratio[good, k])
        else:
            ratio[:, k] = k + 1
    return Timbre(frame_sec, ratio, amp)


def _synth(f0_hz, pos, timbre, sr, nyq=0.45):
    """f0_hz: scalar or [N] Hz; pos: [N] time positions into the timbre (sec)."""
    T, K = timbre.ratio.shape
    frame_t = np.arange(T) * timbre.frame_sec
    sig = np.zeros(len(pos))
    for k in range(K):
        fk = f0_hz * np.interp(pos, frame_t, timbre.ratio[:, k])
        ak = np.where(fk < nyq * sr, np.interp(pos, frame_t, timbre.amp[:, k]), 0.0)
        sig += ak * np.sin(2 * np.pi * np.cumsum(fk) / sr)
    return sig


def render_note(note: Note, timbre: Timbre, sr=44100, time_mode="stretch",
                attack=0.01, release=0.08):
    N = int(round((note.dur + release) * sr))
    tt = np.arange(N) / sr
    span = (len(timbre.ratio) - 1) * timbre.frame_sec
    if time_mode == "stretch":
        pos = tt / max(note.dur, 1e-9) * span
    elif time_mode == "loop":
        pos = np.mod(tt, span + timbre.frame_sec)
    else:
        pos = np.full(N, span / 2)
    pos = np.minimum(pos, span)
    env = np.minimum(1.0, tt / attack)
    rel = tt > note.dur
    env[rel] *= np.maximum(0.0, 1 - (tt[rel] - note.dur) / release)
    return _synth(float(midi_to_hz(note.midi)), pos, timbre, sr) * env * note.vel


def render_melody(notes, timbre_for_note, sr=44100, tail=0.5, **kw):
    """Length is derived from the notes: max(start+dur) + tail."""
    y = np.zeros(int(np.ceil((max(n.start + n.dur for n in notes) + tail) * sr)))
    for i, n in enumerate(notes):
        seg = render_note(n, timbre_for_note(i, n), sr=sr, **kw)
        i0 = int(round(n.start * sr))
        y[i0:i0 + len(seg)] += seg[: len(y) - i0]
    return y


def render_native(timbre: Timbre, f0_track, sr=44100, tail=0.5, release=0.08):
    """Mode A: follow the drawing's own f0 contour. Length = frames * frame_sec + tail."""
    T = len(timbre.ratio)
    good = ~np.isnan(f0_track)
    f0 = np.interp(np.arange(T), np.nonzero(good)[0], f0_track[good])
    dur = T * timbre.frame_sec
    N = int(round(dur * sr))
    pos = np.arange(N) / sr
    frame_t = np.arange(T) * timbre.frame_sec
    sig = _synth(np.interp(pos, frame_t, f0), np.minimum(pos, frame_t[-1]), timbre, sr)
    n_rel = min(int(release * sr), N)
    sig[:int(0.01 * sr)] *= np.linspace(0, 1, int(0.01 * sr))
    sig[N - n_rel:] *= np.linspace(1, 0, n_rel)
    return np.concatenate([sig, np.zeros(int(round(tail * sr)))])


def normalize_peak(y, peak_dbfs=-1.0):
    p = np.max(np.abs(y))
    return y if p == 0 else y * (10 ** (peak_dbfs / 20) / p)


def write_wav(path, y, sr=44100, channels=2):
    pcm = np.round(np.clip(y, -1.0, 1.0) * 32767).astype("<i2")
    if channels == 2:
        pcm = np.repeat(pcm[:, None], 2, axis=1).reshape(-1)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(channels)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(pcm.tobytes())
