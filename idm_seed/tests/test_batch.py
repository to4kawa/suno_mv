import csv
import json
import math

import pytest

from idm_seed.batch import DEFAULT_WEIGHTS, expand, parse_grid, run_batch, score, set_path
from idm_seed.recipe import default_config, verify


def _base(minutes=0.25):
    return {"input": {"kind": "drawn", "seed": 1},
            "melody": {"kind": "generator", "style": "motif", "minutes": minutes, "seed": 4, "options": {}},
            "config": default_config(seed=0, strength=0.5), "gs": None, "channels": 1}


def test_parse_and_expand_grid():
    g = parse_grid(["config.seed=0,1", "config.strength=0.3,0.7", "melody.options.scale=minor,dorian"])
    assert g["config.seed"] == [0, 1] and g["config.strength"] == [0.3, 0.7]
    assert g["melody.options.scale"] == ["minor", "dorian"]
    c = expand(_base(), g)
    assert len(c) == 8
    assert c[0][1]["config"]["seed"] == 0 and c[-1][1]["melody"]["options"]["scale"] == "dorian"
    assert _base()["config"]["seed"] == 0                      # the base recipe is not mutated
    with pytest.raises(ValueError):
        parse_grid(["config.seed"])
    with pytest.raises(KeyError):
        set_path(_base(), "nonexistent.seed", 1)


def test_score_uses_weights_and_rejects_failed_measurements():
    m = {"periodicity": 0.6, "roughness": 0.01, "interpartial_noise": 0.0, "inharmonicity_cents": 100.0}
    assert score(m) == pytest.approx(0.6 - 0.1)
    assert score(m, {"periodicity": 0.0, "roughness": 0.0}) == 0.0
    assert score({"error": "x"}) == -math.inf and score(None) == -math.inf
    assert set(DEFAULT_WEIGHTS) >= {"periodicity", "roughness"}


def test_batch_ranks_within_strength_groups_keeps_top_and_files_verify(tmp_path):
    grid = parse_grid(["config.seed=0,1,2", "config.strength=0.3,0.7"])
    rows = run_batch(_base(), grid, tmp_path / "b", keep=2)
    assert len(rows) == 6
    with open(tmp_path / "b" / "ranking.csv", encoding="utf-8", newline="") as f:
        table = list(csv.DictReader(f))
    assert len(table) == 6 and {r["group"] for r in table} == {"0.3", "0.7"}
    for g in ("0.3", "0.7"):
        sel = [r for r in table if r["group"] == g]
        assert [int(r["rank"]) for r in sel] == [1, 2, 3]
        scores = [float(r["score"]) for r in sel]
        assert scores == sorted(scores, reverse=True)
        assert [bool(r["wav"]) for r in sel] == [True, True, False]   # keep=2
    kept = sorted((tmp_path / "b").glob("*.wav"))
    assert len(kept) == 4
    for w in kept:
        assert verify(w.with_suffix(".json"))["rebuild_matches_sidecar"]
    meta = json.loads((tmp_path / "b" / "batch.json").read_text(encoding="utf-8"))
    assert meta["candidates"] == 6 and meta["group_by"] == "config.strength"


def test_batch_is_deterministic_and_weights_change_the_order(tmp_path):
    grid = parse_grid(["config.seed=0,1,2,3"])
    a = run_batch(_base(), grid, tmp_path / "a", keep=0, group_by="none")
    b = run_batch(_base(), grid, tmp_path / "b", keep=0, group_by="none")
    assert [(r["recipe_id"], r["score"]) for r in a] == [(r["recipe_id"], r["score"]) for r in b]
    by_period = run_batch(_base(), grid, tmp_path / "c", keep=0, group_by="none",
                          weights={"periodicity": 1.0, "roughness": 0.0, "interpartial_noise": 0.0})
    pers = [r["metrics"]["periodicity"] for r in by_period]
    assert pers == sorted(pers, reverse=True)                  # pure periodicity weight -> sorted by it


def test_guards(tmp_path):
    with pytest.raises(ValueError, match="max"):
        run_batch(_base(), parse_grid(["config.seed=0,1,2"]), tmp_path / "x", max_candidates=2)
    native = _base(); native["melody"] = {"kind": "native"}
    with pytest.raises(ValueError, match="melody"):
        run_batch(native, {}, tmp_path / "y")
