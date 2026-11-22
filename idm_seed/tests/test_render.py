import wave

import numpy as np

from idm_seed import Note, SeedBuilder, SeedConfig, write_wav
from idm_seed.draw import drawn_spectrogram
from metrics import interpartial_noise_fraction, note_inharmonicity_cents, periodicity

SR = 44100
MEL = [57, 60, 64, 62, 60, 67, 65, 64, 69, 67, 64, 60, 62, 65, 64, 57]
NOTES = [Note(i * 0.5, 0.45, m, 0.9) for i, m in enumerate(MEL)]


def _builder(s=0.5, seed=3, **kw):
    S, freqs, fs = drawn_spectrogram(seed=1)
    return SeedBuilder(S, freqs, fs, SeedConfig(strength=s, seed=seed, **kw)), (S, freqs, fs)


def _inharm(y, notes=NOTES):
    r = [note_inharmonicity_cents(y, SR, n) for n in notes]
    return np.array([x[0] for x in r]), np.array([x[1] for x in r])


def test_pitch_follows_score():
    b, _ = _builder()
    _, f0err = _inharm(b.melody(NOTES))
    assert np.nanmax(f0err) < 5.0, f0err


def test_strength_changes_the_rendered_sound_monotonically_and_by_a_lot():
    means = []
    for s in (0.0, 0.25, 0.5, 0.75, 1.0):
        b, _ = _builder(s=s)
        d, _ = _inharm(b.melody(NOTES))
        means.append(float(np.nanmean(d)))
    assert all(b < a for a, b in zip(means, means[1:])), means
    assert means[0] - means[-1] > 40.0, means      # audible-scale change, in cents
    assert means[-1] > 10.0, means                 # never fully clean


def test_notes_do_not_share_one_quirk():
    b, _ = _builder(s=0.5)
    same_pitch = [Note(i * 0.5, 0.45, 60, 0.9) for i in range(8)]
    d, _ = _inharm(b.melody(same_pitch), same_pitch)
    assert np.nanstd(d) > 5.0, d


def test_deterministic_and_seed_sensitive():
    a = _builder(seed=3)[0].melody(NOTES)
    b = _builder(seed=3)[0].melody(NOTES)
    c = _builder(seed=4)[0].melody(NOTES)
    assert np.array_equal(a, b) and not np.array_equal(a, c)


def test_length_comes_from_input():
    b, (S, _f, fs) = _builder()
    y = b.melody(NOTES)
    assert len(y) == int(np.ceil((max(n.start + n.dur for n in NOTES) + 0.5) * SR))
    yn = b.native()
    assert abs(len(yn) - round((S.shape[0] * fs + 0.5) * SR)) <= 1
    short = b.melody(NOTES[:3])
    assert len(short) < len(y)


def test_wav_format_and_levels(tmp_path):
    b, _ = _builder()
    y = b.melody(NOTES)
    p = tmp_path / "x.wav"
    write_wav(p, y, SR, 2)
    with wave.open(str(p)) as w:
        assert (w.getframerate(), w.getnchannels(), w.getsampwidth()) == (SR, 2, 2)
    assert np.all(np.isfinite(y)) and abs(20 * np.log10(np.max(np.abs(y))) + 1.0) < 0.01
    assert np.sqrt(np.mean(y[-int(0.05 * SR):] ** 2)) < 1e-6   # ends in silence, no click


def test_novelty_filter_accepts_far_reference_and_exhausts_on_close_one():
    S, freqs, fs = drawn_spectrogram(seed=1)
    far = np.zeros(S.shape[1]); far[-3:] = 1.0                       # nothing like the input
    b = SeedBuilder(S, freqs, fs, SeedConfig(seed=3), gs_spectrum=far)
    b.melody(NOTES[:4])
    assert all(ok and att == 1 for _k, att, _s, ok in b.novelty_log), b.novelty_log
    close = S.mean(axis=0)
    b2 = SeedBuilder(S, freqs, fs, SeedConfig(seed=3, max_similarity=0.3, max_retries=2), gs_spectrum=close)
    b2.melody(NOTES[:2])
    assert all((not ok) and att == 3 for _k, att, _s, ok in b2.novelty_log), b2.novelty_log


def test_long_melody_generator_is_exact_and_deterministic():
    from idm_seed.cli import make_long_melody
    a, b = make_long_melody(10, 0), make_long_melody(10, 0)
    assert [(n.start, n.dur, n.midi) for n in a] == [(n.start, n.dur, n.midi) for n in b]
    assert max(n.start + n.dur for n in a) <= 600.0
    assert max(n.start + n.dur for n in a) > 595.0        # fills the requested length
    assert all(x.start <= y.start for x, y in zip(a, a[1:]))
    assert make_long_melody(1, 0)[-1].start + make_long_melody(1, 0)[-1].dur <= 60.0


def test_no_crackle_between_partials():
    b, _ = _builder(s=0.5)
    y = b.melody(NOTES)
    fr = [interpartial_noise_fraction(y, SR, n) for n in NOTES]
    assert np.mean(fr) < 0.005, fr     # old stretched rendering measured ~0.03 before smoothing


def test_strength_changes_pitch_clarity_a_lot_and_full_snap_is_periodic():
    means = []
    for s in (0.0, 0.5, 1.0):
        b, _ = _builder(s=s)
        y = b.melody(NOTES)
        means.append(np.mean([periodicity(y, SR, n) for n in NOTES]))
    # measured over 4 different drawings: s=0 -> 0.32..0.41, s=1 -> 0.68..0.77
    assert means[0] < 0.5 and means[2] > 0.6 and means[2] - means[0] > 0.25, means
    assert means[0] < means[1] < means[2], means
    S, freqs, fs = drawn_spectrogram(seed=1)
    ref = SeedBuilder(S, freqs, fs, SeedConfig(strength=1.0, seed=3, variation=0.0, ramp=0.0,
                                               p_snap=1.0, p_leave=0.0, drift_cents=0.0))
    yr = ref.melody(NOTES)
    assert np.mean([periodicity(yr, SR, n) for n in NOTES]) > 0.9
