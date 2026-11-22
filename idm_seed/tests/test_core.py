import numpy as np

from idm_seed.core import (GlitchPlan, apply_partial_correction, detect_harmonic_structure,
                           make_glitch_plan)
from idm_seed.draw import drawn_spectrogram

K = 9


def _dev_cents(hs, t_slice=slice(None)):
    k = np.arange(1, K + 1)[None, :]
    d = np.abs(1200 * np.log2(hs.partial_hz[t_slice] / (k * hs.f0[t_slice, None])))
    return float(np.nanmean(d[:, 1:]))


def _setup(seed=1):
    S, freqs, fs = drawn_spectrogram(seed=seed, n_partials=K)
    return S, freqs, detect_harmonic_structure(S, freqs, n_harm=K)


def test_detects_f0_and_partials():
    S, freqs, hs = _setup()
    voiced = ~np.isnan(hs.f0)
    assert voiced.mean() > 0.9                       # frames where the drawn fundamental drops out stay unvoiced
    assert np.max(np.abs(hs.f0[voiced] - 440.0)) < 1.0
    assert np.median(np.sum(~np.isnan(hs.partial_hz), axis=1)) >= 7


def test_strength_zero_is_identity_and_deterministic():
    S, freqs, hs = _setup()
    plan = make_glitch_plan(K, seed=5)
    assert np.allclose(apply_partial_correction(S, freqs, hs, 0.0, plan), S)
    a = apply_partial_correction(S, freqs, hs, 0.6, plan)
    b = apply_partial_correction(S, freqs, hs, 0.6, make_glitch_plan(K, seed=5))
    assert np.array_equal(a, b)
    c = apply_partial_correction(S, freqs, hs, 0.6, make_glitch_plan(K, seed=6))
    assert not np.array_equal(a, c)


def test_uniform_plan_deviation_decreases_but_not_to_zero_below_one():
    S, freqs, hs = _setup()
    plan = GlitchPlan(np.ones(K), np.zeros(K))
    devs = []
    for s in np.linspace(0, 1, 6):
        out = apply_partial_correction(S, freqs, hs, s, plan)
        devs.append(_dev_cents(detect_harmonic_structure(out, freqs, n_harm=K)))
    assert all(b < a for a, b in zip(devs, devs[1:])), devs
    assert devs[0] > 100, devs          # drawn input really is far from harmonic
    assert devs[-2] > 5, devs           # s=0.8 still clearly not clean
    assert devs[-1] < 10, devs          # s=1 with g=1 is (nearly) fully harmonic


def test_left_alone_partials_never_move_without_drift():
    S, freqs, hs = _setup()
    plan = GlitchPlan(np.array([1, 1, 0, 1, 0, 1, 1, 1, 1.0]), np.zeros(K))
    out = apply_partial_correction(S, freqs, hs, 1.0, plan)
    hs2 = detect_harmonic_structure(out, freqs, n_harm=K)
    for k in (3, 5):  # 1-based slots left alone
        a, b = hs.partial_hz[:, k - 1], hs2.partial_hz[:, k - 1]
        ok = ~np.isnan(a) & ~np.isnan(b)
        assert np.nanmedian(np.abs(1200 * np.log2(b[ok] / a[ok]))) < 3.0


def test_frame_energy_preserved():
    S, freqs, hs = _setup()
    out = apply_partial_correction(S, freqs, hs, 0.7, make_glitch_plan(K, seed=2))
    e0, e1 = np.sum(S ** 2, axis=1), np.sum(out ** 2, axis=1)
    assert np.max(np.abs(e1 - e0) / e0) < 0.02
