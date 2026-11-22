import numpy as np
import pytest

from idm_seed import Note, SeedBuilder, SeedConfig, write_wav
from idm_seed.draw import drawn_spectrogram
from idm_seed.metrics import analyze, analyze_wav, load_wav_mono

SR = 44100
NOTES = [Note(i * 0.5, 0.45, m, 0.9) for i, m in enumerate([57, 60, 64, 62, 60, 67, 65, 64, 69, 67, 64, 60])]


def _seed(s):
    S, f, fs = drawn_spectrogram(seed=1)
    return SeedBuilder(S, f, fs, SeedConfig(strength=s, seed=3)).melody(NOTES)


def test_wav_roundtrip_matches_in_memory_for_stereo_and_mono(tmp_path):
    y = _seed(0.5)
    ref = analyze(y, SR, NOTES)
    for ch in (1, 2):
        p = tmp_path / f"x{ch}.wav"
        write_wav(p, y, SR, ch)
        got = analyze_wav(p, NOTES)
        for k in ("inharmonicity_cents", "periodicity", "roughness"):
            assert got[k] == pytest.approx(ref[k], rel=0.02, abs=1e-3), (k, got[k], ref[k])
        assert got["notes_analyzed"] == ref["notes_analyzed"] == len(NOTES)


def test_metrics_move_the_expected_way_with_strength():
    lo, hi = analyze(_seed(0.0), SR, NOTES), analyze(_seed(1.0), SR, NOTES)
    assert lo["inharmonicity_cents"] > hi["inharmonicity_cents"] + 30
    assert hi["periodicity"] > lo["periodicity"] + 0.2
    assert hi["roughness"] < lo["roughness"]
    assert lo["f0_error_cents_max"] < 5


def test_sampling_is_deterministic_and_bounded():
    y = _seed(0.5)
    assert analyze(y, SR, NOTES, max_notes=5) == analyze(y, SR, NOTES, max_notes=5)
    assert analyze(y, SR, NOTES, max_notes=5)["notes_analyzed"] == 5


def test_no_measurable_note_is_an_error_not_a_silent_zero():
    with pytest.raises(ValueError):
        analyze(_seed(0.5), SR, [Note(0, 0.1, 60)])


def test_load_wav_mono_rejects_non_16bit(tmp_path):
    import wave
    p = tmp_path / "x.wav"
    with wave.open(str(p), "wb") as w:
        w.setnchannels(1); w.setsampwidth(1); w.setframerate(8000); w.writeframes(b"\x80" * 100)
    with pytest.raises(ValueError):
        load_wav_mono(p)
