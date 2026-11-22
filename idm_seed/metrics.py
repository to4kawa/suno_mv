"""Objective measurements of rendered seeds. They are proxies, not taste: judge with them, then listen.

note_inharmonicity_cents  mean |cents| of partials from integer multiples of f0 (glitch amount)
periodicity               normalised autocorrelation at the note period (pitch clarity)
interpartial_noise_fraction  power between partials (flicker / crackle)
roughness                 Plomp-Levelt beating between close partials
analyze()                 medians over analysable notes; analyze_wav() reads a WAV first
"""
import wave

import numpy as np

from .core import find_f0_lowest, find_partials
from .render import midi_to_hz


def note_inharmonicity_cents(y, sr, note, n_harm=10, nfft=8192):
    """Mean |cents| between each measured partial and its nearest integer multiple,
    measured on the middle of the note. Also returns the number of partials found."""
    f_note = float(midi_to_hz(note.midi))
    mid = int((note.start + note.dur / 2) * sr)
    seg = y[mid - nfft // 2: mid + nfft // 2]
    spec = np.abs(np.fft.rfft(seg * np.hanning(len(seg))))
    freqs = np.fft.rfftfreq(len(seg), 1 / sr)
    f0 = find_f0_lowest(spec, freqs, f_note * 0.9, f_note * 1.1, 0.1)
    if np.isnan(f0):
        return float("nan"), float("nan"), 0
    parts = find_partials(spec, freqs, f0, n_harm, 0.05)
    devs = [abs(1200 * np.log2(hz / (k * f0))) for k, (hz, _a) in parts.items() if k >= 2]
    f0_err = abs(1200 * np.log2(f0 / f_note))
    return (float(np.mean(devs)) if devs else float("nan")), f0_err, len(devs)


def interpartial_noise_fraction(y, sr, note, nfft=8192, guard_hz=25.0, top=14):
    """Fraction of mid-note spectral power farther than guard_hz from the strongest spectral
    peaks. Steady sinusoidal partials -> ~0; fast amplitude flicker spreads power between them."""
    mid = int((note.start + note.dur / 2) * sr)
    seg = y[mid - nfft // 2: mid + nfft // 2]
    p = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2
    f = np.fft.rfftfreq(len(seg), 1 / sr)
    pk = np.nonzero((p[1:-1] > p[:-2]) & (p[1:-1] >= p[2:]))[0] + 1
    pk = pk[np.argsort(p[pk])[::-1][:top]]
    keep = np.zeros(len(p), bool)
    for j in pk:
        keep |= np.abs(f - f[j]) <= guard_hz
    return float(p[~keep].sum() / p.sum())


def periodicity(y, sr, note, n=4096):
    """Normalised autocorrelation peak near the note period (1 = perfectly periodic, i.e. a
    harmonic tone with clear pitch; inharmonic partials lower it)."""
    f = float(midi_to_hz(note.midi))
    mid = int((note.start + note.dur / 2) * sr)
    x = y[mid - n // 2: mid + n // 2]
    x = x - x.mean()
    lag0 = sr / f
    best = 0.0
    for lag in range(int(lag0 * 0.97), int(lag0 * 1.03) + 1):
        a, b = x[:-lag], x[lag:]
        best = max(best, float(a @ b / (np.linalg.norm(a) * np.linalg.norm(b) + 1e-12)))
    return best


def roughness(y, sr, note, nfft=8192, rel=0.02):
    """Plomp-Levelt / Sethares sensory dissonance of the partials in the middle of a note.
    Higher = more beating between close partials (rough, crackly). Also returns the smallest
    spacing between adjacent partials in units of the note f0."""
    mid = int((note.start + note.dur / 2) * sr)
    seg = y[mid - nfft // 2: mid + nfft // 2]
    sp = np.abs(np.fft.rfft(seg * np.hanning(len(seg))))
    f = np.fft.rfftfreq(len(seg), 1 / sr)
    pk = np.nonzero((sp[1:-1] > sp[:-2]) & (sp[1:-1] >= sp[2:]) & (sp[1:-1] >= rel * sp.max()))[0] + 1
    fr, am = f[pk], sp[pk] / sp.max()
    d = 0.0
    for i in range(len(fr)):
        for j in range(i + 1, len(fr)):
            lo = min(fr[i], fr[j]); df = abs(fr[i] - fr[j])
            s = 0.24 / (0.0207 * lo + 18.96)
            d += am[i] * am[j] * (np.exp(-3.5 * s * df) - np.exp(-5.75 * s * df))
    f0 = float(midi_to_hz(note.midi))
    gaps = np.diff(np.sort(fr)) / f0
    return float(d), float(gaps.min()) if len(gaps) else float("nan")


def load_wav_mono(path):
    """16-bit PCM WAV -> (float64 mono in [-1, 1], sample rate). Channels are averaged."""
    with wave.open(str(path)) as w:
        if w.getsampwidth() != 2:
            raise ValueError("only 16-bit PCM WAV is supported")
        ch, sr, raw = w.getnchannels(), w.getframerate(), w.readframes(w.getnframes())
    y = np.frombuffer(raw, dtype="<i2").astype(np.float64) / 32768.0
    return (y.reshape(-1, ch).mean(axis=1) if ch > 1 else y), sr


def analyze(y, sr, notes, max_notes=64, min_dur=0.35):
    """Median of each metric over notes long enough to measure (>= min_dur s). At most
    max_notes notes, picked evenly (deterministic), keep the cost bounded for 10-minute seeds."""
    usable = [n for n in notes if n.dur >= min_dur and int((n.start + n.dur / 2) * sr) + 4096 < len(y)]
    if not usable:
        raise ValueError(f"no note of at least {min_dur}s to analyse")
    if len(usable) > max_notes:
        usable = [usable[i] for i in np.linspace(0, len(usable) - 1, max_notes).round().astype(int)]
    inh, f0e, per, noi, rgh = [], [], [], [], []
    for n in usable:
        d, e, _c = note_inharmonicity_cents(y, sr, n)
        inh.append(d); f0e.append(e)
        per.append(periodicity(y, sr, n))
        noi.append(interpartial_noise_fraction(y, sr, n))
        rgh.append(roughness(y, sr, n)[0])
    med = lambda v: float(np.nanmedian(v))
    return {"notes_analyzed": len(usable), "inharmonicity_cents": med(inh),
            "periodicity": med(per), "interpartial_noise": med(noi), "roughness": med(rgh),
            "f0_error_cents_max": float(np.nanmax(f0e))}


def analyze_wav(path, notes, **kw):
    y, sr = load_wav_mono(path)
    return analyze(y, sr, notes, **kw)
