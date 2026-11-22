"""Measure rendered audio: how far partials sit from integer multiples of the note's f0."""
import numpy as np

from idm_seed.core import find_f0_lowest, find_partials
from idm_seed.render import midi_to_hz


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
