import numpy as np
import pytest

from idm_seed.melody import SCALES, make_classic_melody, make_melody, make_motif_melody


def _seq(notes):
    return [(round(n.start, 4), round(n.dur, 4), n.midi) for n in notes]


def test_deterministic_and_seeds_differ():
    assert _seq(make_motif_melody(1, 3)) == _seq(make_motif_melody(1, 3))
    seqs = [[n.midi for n in make_motif_melody(2, s)][:40] for s in range(6)]
    diffs = [np.mean(np.array(a) != np.array(b)) for i, a in enumerate(seqs) for b in seqs[i + 1:]]
    assert min(diffs) > 0.5, diffs


def test_length_is_exact_for_any_minutes():
    for minutes in (0.5, 1, 3):
        ns = make_motif_melody(minutes, 1, bpm=100)
        end = max(n.start + n.dur for n in ns)
        assert end <= minutes * 60 and end > minutes * 60 - 8 * 60 / 100 * 1.0
        assert all(x.start <= y.start for x, y in zip(ns, ns[1:]))


def test_motif_is_more_stepwise_and_less_leapy_than_classic():
    def shape(fn):
        iv = np.concatenate([np.abs(np.diff([n.midi for n in fn(s)])) for s in range(6)])
        return (iv <= 2).mean(), (iv >= 5).mean()
    st_m, lp_m = shape(lambda s: make_motif_melody(2, s))
    st_c, lp_c = shape(lambda s: make_classic_melody(2, s))
    assert st_m > 0.45 and st_m > st_c + 0.2, (st_m, st_c)      # measured 0.54 vs 0.23
    assert lp_m < 0.35 and lp_m < lp_c - 0.2, (lp_m, lp_c)      # measured 0.23 vs 0.65


@pytest.mark.parametrize("scale", sorted(SCALES))
def test_notes_stay_in_key_and_range(scale):
    root = 62
    ns = make_motif_melody(1, 2, root=root, scale=scale)
    allowed = {(root + s) % 12 for s in SCALES[scale]}
    assert all(int(n.midi) % 12 in allowed for n in ns)
    assert all(root <= n.midi <= root + 19 for n in ns)


def test_sections_repeat_so_cover_has_a_skeleton_and_phrases_cadence():
    ns = make_motif_melody(2, 4, bpm=120, rest_prob=0.0)
    blocks = {}
    for n in ns:
        blocks.setdefault(int(n.start // 4.0), []).append(n)
    tup = [tuple(round(x.midi) for x in v) for v in blocks.values()]
    assert 1 - len(set(tup)) / len(tup) > 0.6
    full = [v for v in blocks.values() if v[-1].start + v[-1].dur > 3.5 + int(v[0].start // 4.0) * 4.0 - 1e-9]
    ends = {round(v[-1].midi) % 12 for v in blocks.values() if len(v) > 3}
    assert ends <= {57 % 12, (57 + 7) % 12}, ends   # root A or fifth E


def test_dispatch_and_unknown_style():
    assert _seq(make_melody("classic", 1, 2)) == _seq(make_classic_melody(1, 2))
    with pytest.raises(ValueError):
        make_melody("nope", 1)
