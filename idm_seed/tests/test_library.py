import json

import numpy as np
import pytest

from idm_seed import library as L
from idm_seed.recipe import default_config, write_seed


def _first_id(lib):
    return next(iter(L.load(lib)[0]))


def _seed(tmp_path, name, strength, seed=0):
    r = {"input": {"kind": "drawn", "seed": 1},
         "melody": {"kind": "generator", "style": "motif", "minutes": 0.2, "seed": 3, "options": {}},
         "config": default_config(strength=strength, seed=seed), "gs": None, "channels": 1}
    return write_seed(tmp_path / f"{name}.wav", r), tmp_path / f"{name}.json"


def test_add_note_list_and_prefix_ids(tmp_path):
    lib = tmp_path / "lib" / "library.jsonl"
    side, sj = _seed(tmp_path, "a", 0.5)
    rec = L.add_seed(lib, sj, tags=["demo"])
    assert rec["id"] == side["recipe_id"] and rec["tags"] == ["demo"]
    L.add_note(lib, side["recipe_id"][:6], rating=4, mode="cover", text="構造を持ってけた")
    L.add_note(lib, side["recipe_id"], rating=2, mode="extend", text="つながりが悪い")
    rows, broken = L.table(lib)
    assert broken == 0 and len(rows) == 1
    assert rows[0]["rating"] == 3.0 and rows[0]["n_notes"] == 2
    assert rows[0]["last_note"]["text"] == "つながりが悪い"            # Japanese survives the round trip
    assert "\\" not in rows[0]["wav"]


def test_unknown_or_ambiguous_id_and_bad_rating(tmp_path):
    lib = tmp_path / "library.jsonl"
    _s, sj = _seed(tmp_path, "a", 0.5)
    L.add_seed(lib, sj)
    with pytest.raises(KeyError, match="no seed"):
        L.add_note(lib, "deadbeef0000", rating=3)
    with pytest.raises(ValueError):
        L.add_note(lib, _first_id(lib), rating=9)
    with pytest.raises(KeyError):
        L.add_note(lib, "ab", rating=3)                               # prefix shorter than 4


def test_append_only_and_broken_lines_are_ignored(tmp_path):
    lib = tmp_path / "library.jsonl"
    _s, sj = _seed(tmp_path, "a", 0.5)
    L.add_seed(lib, sj)
    before = lib.read_bytes()
    with open(lib, "a", encoding="utf-8") as f:
        f.write("{not json\n")
        f.write(json.dumps({"type": "mystery", "id": "x"}) + "\n")
    L.add_note(lib, _first_id(lib), rating=5)
    assert lib.read_bytes().startswith(before)                        # earlier bytes never rewritten
    seeds, notes, broken = L.load(lib)
    assert len(seeds) == 1 and len(notes) == 1 and broken == 2
    assert L.load(tmp_path / "nope.jsonl") == ({}, [], 0)


def test_re_adding_a_seed_keeps_one_row_latest_wins(tmp_path):
    lib = tmp_path / "library.jsonl"
    _s, sj = _seed(tmp_path, "a", 0.5)
    L.add_seed(lib, sj, tags=["first"]); L.add_seed(lib, sj, tags=["second"])
    rows, _b = L.table(lib)
    assert len(rows) == 1 and rows[0]["tags"] == ["second"]


def test_stats_needs_enough_ratings_then_recovers_a_planted_relation(tmp_path):
    lib = tmp_path / "library.jsonl"
    ids = []
    for i, s in enumerate((0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.5)):
        side, sj = _seed(tmp_path, f"s{i}", s, seed=i)
        L.add_seed(lib, sj); ids.append((side["recipe_id"], side["metrics"]["periodicity"]))
    assert "need at least" in L.stats(lib)["status"]
    # plant: owner rating rises with periodicity (rank order of periodicity -> rating 1..5, ties impossible here)
    order = np.argsort([p for _i, p in ids])
    for rank, idx in enumerate(order):
        L.add_note(lib, ids[idx][0], rating=1 + min(4, rank * 5 // len(ids)))
    st = L.stats(lib)
    assert st["status"] == "ok" and st["rated_seeds"] == 7
    assert st["spearman"]["periodicity"] > 0.9
    assert st["suggested_weights"]["periodicity"] > 0 and max(abs(v) for v in st["suggested_weights"].values()) == pytest.approx(1.0)


def test_spearman_known_values():
    assert L.spearman([1, 2, 3, 4], [10, 20, 30, 40]) == pytest.approx(1.0)
    assert L.spearman([1, 2, 3, 4], [4, 3, 2, 1]) == pytest.approx(-1.0)
    assert np.isnan(L.spearman([1, 1, 1], [1, 2, 3]))
