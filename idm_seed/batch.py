"""Batch: render a grid of candidate seeds, score them with the objective metrics, keep the best.

The score is a *playability* index (clear pitch, little beating, little noise), NOT a measure of
taste. Strength is a creative choice, so rank inside groups (default: per config.strength) instead
of letting the score always prefer the cleanest sound. Weights are uncalibrated placeholders:
tune them from your own ratings (library.py) and pass them with --weights.
"""
from __future__ import annotations

import copy
import csv
import itertools
import json
import math
import pathlib

from .metrics import analyze
from .recipe import build_seed, recipe_id, write_seed

DEFAULT_WEIGHTS = {"periodicity": 1.0, "roughness": -10.0, "interpartial_noise": -10.0,
                   "inharmonicity_cents": 0.0}
METRIC_KEYS = ["inharmonicity_cents", "periodicity", "interpartial_noise", "roughness",
               "f0_error_cents_max"]


def set_path(d, dotted, value):
    keys = dotted.split(".")
    for k in keys[:-1]:
        if not isinstance(d.get(k), dict):
            raise KeyError(f"grid path not found in recipe: {dotted!r} (missing {k!r})")
        d = d[k]
    d[keys[-1]] = value          # new option keys (e.g. melody.options.root) are allowed


def _val(text):
    try:
        return json.loads(text)
    except ValueError:
        return text


def parse_grid(items):
    """['config.strength=0.3,0.5', 'config.seed=0,1'] -> {path: [values]} in the given order."""
    grid = {}
    for it in items or []:
        if "=" not in it:
            raise ValueError(f"grid entry needs path=v1,v2,...: {it!r}")
        path, vals = it.split("=", 1)
        grid[path.strip()] = [_val(v.strip()) for v in vals.split(",") if v.strip() != ""]
        if not grid[path.strip()]:
            raise ValueError(f"grid entry has no values: {it!r}")
    return grid


def expand(base, grid):
    paths = list(grid)
    out = []
    for combo in itertools.product(*(grid[p] for p in paths)):
        r = copy.deepcopy(base)
        for p, v in zip(paths, combo):
            set_path(r, p, v)
        out.append((dict(zip(paths, combo)), r))
    return out


def score(metrics, weights=None):
    w = {**DEFAULT_WEIGHTS, **(weights or {})}
    if not metrics or "error" in metrics:
        return -math.inf
    return float(sum(w[k] * metrics[k] for k in w if k in metrics))


def run_batch(base, grid, out_dir, keep=3, group_by="config.strength", weights=None,
              max_candidates=200):
    """Returns the ranked rows (all candidates). Writes ranking.csv, batch.json and the kept
    candidates (WAV + sidecar) into out_dir. Candidates are rendered twice only if kept: the first
    pass keeps metrics only, so a 10-minute grid does not hold every waveform in memory."""
    cands = expand(base, grid) if grid else [({}, copy.deepcopy(base))]
    if len(cands) > max_candidates:
        raise ValueError(f"{len(cands)} candidates > max {max_candidates}; shrink the grid or raise --max-candidates")
    out_dir = pathlib.Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    rows = []
    for vals, recipe in cands:
        y, notes = build_seed(recipe, out_dir)
        if not notes:
            raise ValueError("batch ranking needs a melody (notes): the 'native' mode has nothing to measure")
        try:
            m = analyze(y, recipe["config"]["sr"], notes)
        except ValueError as e:
            m = {"error": str(e)}
        rows.append({"values": vals, "recipe": recipe, "recipe_id": recipe_id(recipe),
                     "metrics": m, "score": score(m, weights),
                     "group": str(vals.get(group_by)) if group_by in vals else "all"})
    groups = {}
    for r in rows:
        groups.setdefault(r["group"], []).append(r)
    for g in groups.values():
        g.sort(key=lambda r: (-r["score"], r["recipe_id"]))     # ties broken by id: stable order
        for i, r in enumerate(g, 1):
            r["rank"] = i
            r["wav"] = ""
            if i <= keep and r["score"] > -math.inf:
                name = f"{_safe(r['group'])}_{i:02d}_{r['recipe_id']}.wav"
                write_seed(out_dir / name, r["recipe"])
                r["wav"] = name
    rows.sort(key=lambda r: (r["group"], r["rank"]))
    _write_csv(out_dir / "ranking.csv", rows, list(grid))
    (out_dir / "batch.json").write_text(json.dumps(
        {"base_recipe": base, "grid": grid, "keep": keep, "group_by": group_by,
         "weights": {**DEFAULT_WEIGHTS, **(weights or {})}, "candidates": len(rows)},
        indent=1, ensure_ascii=False), encoding="utf-8")
    return rows


def _safe(s):
    return "".join(c if c.isalnum() or c in "._-" else "_" for c in str(s))


def _write_csv(path, rows, grid_paths):
    cols = ["group", "rank", "score"] + METRIC_KEYS + grid_paths + ["recipe_id", "wav"]
    with open(path, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(cols)
        for r in rows:
            m = r["metrics"] if "error" not in r["metrics"] else {}
            w.writerow([r["group"], r["rank"], f"{r['score']:.4f}"]
                       + [f"{m[k]:.5g}" if k in m else "" for k in METRIC_KEYS]
                       + [r["values"].get(p, "") for p in grid_paths] + [r["recipe_id"], r["wav"]])
