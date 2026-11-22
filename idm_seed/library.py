"""Seed library: an append-only JSON-lines file of seeds and the owner's notes about how each
one did in Suno (or any other engine). Nothing is ever rewritten: later lines win on read.

{"type": "seed", "id": recipe_id, "wav": "path", "added_utc": ..., "metrics": {...}, "summary": {...}}
{"type": "note", "id": recipe_id, "utc": ..., "rating": 1-5, "engine": "suno", "mode": "cover", "text": "...", "tags": []}
"""
from __future__ import annotations

import datetime
import json
import pathlib

import numpy as np

from .recipe import read_sidecar

METRICS = ["inharmonicity_cents", "periodicity", "interpartial_noise", "roughness"]
MIN_RATED_FOR_STATS = 5


def _now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


def _append(lib, rec):
    lib = pathlib.Path(lib)
    lib.parent.mkdir(parents=True, exist_ok=True)
    with open(lib, "a", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(rec, ensure_ascii=False, sort_keys=True) + "\n")


def load(lib):
    """-> (seeds {id: rec}, notes [rec], broken_line_count). Missing file = empty library."""
    seeds, notes, broken = {}, [], 0
    p = pathlib.Path(lib)
    if not p.exists():
        return seeds, notes, 0
    for line in p.read_text(encoding="utf-8", errors="replace").splitlines():
        if not line.strip():
            continue
        try:
            rec = json.loads(line)
            kind = rec["type"]
            rec["id"]
        except (ValueError, KeyError, TypeError):
            broken += 1
            continue
        if kind == "seed":
            seeds[rec["id"]] = rec
        elif kind == "note":
            notes.append(rec)
        else:
            broken += 1
    return seeds, notes, broken


def _posix(path, lib):
    p = pathlib.Path(str(path).replace("\\", "/"))
    try:
        return p.resolve().relative_to(pathlib.Path(lib).resolve().parent).as_posix()
    except ValueError:
        return p.resolve().as_posix()


def add_seed(lib, sidecar_file, tags=()):
    side = read_sidecar(sidecar_file)
    sidecar_file = pathlib.Path(sidecar_file)
    wav = sidecar_file.parent / side["output"]["wav"]
    r, mel = side["recipe"], side["recipe"]["melody"]
    rec = {"type": "seed", "id": side["recipe_id"], "wav": _posix(wav, lib), "added_utc": _now(),
           "metrics": side.get("metrics"), "tags": list(tags),
           "summary": {"strength": r["config"].get("strength"), "seed": r["config"].get("seed"),
                       "melody": mel.get("style", mel["kind"]), "seconds": side["output"]["seconds"],
                       "options": mel.get("options", {})}}
    _append(lib, rec)
    return rec


def resolve_id(seeds, key):
    if key in seeds:
        return key
    hits = [i for i in seeds if i.startswith(key)]
    if len(hits) == 1 and len(key) >= 4:
        return hits[0]
    if not hits:
        raise KeyError(f"no seed with id {key!r}; add it first (library add --sidecar ...)")
    raise KeyError(f"id prefix {key!r} is ambiguous: {', '.join(sorted(hits))}")


def add_note(lib, key, rating=None, engine="suno", mode=None, text="", tags=()):
    seeds, _n, _b = load(lib)
    sid = resolve_id(seeds, key)
    if rating is not None and not (isinstance(rating, int) and 1 <= rating <= 5):
        raise ValueError("rating must be an integer 1-5")
    rec = {"type": "note", "id": sid, "utc": _now(), "rating": rating, "engine": engine,
           "mode": mode, "text": text, "tags": list(tags)}
    _append(lib, rec)
    return rec


def table(lib):
    """One row per seed: its record plus mean rating, note count and the latest note."""
    seeds, notes, broken = load(lib)
    rows = []
    for sid, s in seeds.items():
        ns = [n for n in notes if n["id"] == sid]
        rated = [n["rating"] for n in ns if n.get("rating") is not None]
        rows.append({**s, "n_notes": len(ns),
                     "rating": float(np.mean(rated)) if rated else None,
                     "last_note": ns[-1] if ns else None})
    return rows, broken


def _ranks(v):
    v = np.asarray(v, float)
    order = np.argsort(v, kind="mergesort")
    r = np.empty(len(v))
    r[order] = np.arange(len(v), dtype=float)
    for u in np.unique(v):            # average ranks over ties
        m = v == u
        r[m] = r[m].mean()
    return r


def spearman(a, b):
    ra, rb = _ranks(a), _ranks(b)
    if ra.std() == 0 or rb.std() == 0:
        return float("nan")
    return float(np.corrcoef(ra, rb)[0, 1])


def stats(lib, min_rated=MIN_RATED_FOR_STATS):
    """Rank correlation between each metric and the owner's mean rating, and a transparent
    weight suggestion (corr / std, scaled so the largest |weight| is 1). Needs rated seeds that
    have metrics; with few ratings it only reports how many are missing."""
    rows, _b = table(lib)
    use = [r for r in rows if r["rating"] is not None and r.get("metrics") and "error" not in r["metrics"]]
    out = {"rated_seeds": len(use), "needed": min_rated}
    if len(use) < min_rated:
        out["status"] = f"need at least {min_rated} rated seeds with metrics (have {len(use)})"
        return out
    y = [r["rating"] for r in use]
    corr, w = {}, {}
    for k in METRICS:
        x = [r["metrics"][k] for r in use]
        corr[k] = spearman(x, y)
        sd = float(np.std(x))
        w[k] = corr[k] / sd if sd > 0 and not np.isnan(corr[k]) else 0.0
    m = max(abs(v) for v in w.values()) or 1.0
    out.update(status="ok", spearman=corr, suggested_weights={k: v / m for k, v in w.items()},
               caveat="rank correlations over a few ratings are noisy; treat as a hint, not a fit")
    return out
