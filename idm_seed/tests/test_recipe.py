import hashlib
import json

import numpy as np
import pytest

from idm_seed.draw import drawn_spectrogram
from idm_seed.inputs import notes_to_json
from idm_seed.melody import make_melody
from idm_seed.recipe import (build_seed, default_config, file_ref, read_sidecar, recipe_id,
                             verify, write_seed)


def _gen_recipe(seed=2, strength=0.5, minutes=0.4):
    return {"input": {"kind": "drawn", "seed": seed},
            "melody": {"kind": "generator", "style": "motif", "minutes": minutes, "seed": seed,
                       "options": {"root": 57, "scale": "minor", "bpm": 120}},
            "config": default_config(strength=strength, seed=seed), "gs": None, "channels": 1}


def test_build_is_deterministic():
    a, _ = build_seed(_gen_recipe())
    b, _ = build_seed(_gen_recipe())
    assert np.array_equal(a, b)
    c, _ = build_seed(_gen_recipe(strength=0.9))
    assert not np.array_equal(a, c)


def test_sidecar_contents_and_bitexact_verify(tmp_path):
    wav = tmp_path / "seed.wav"
    side = write_seed(wav, _gen_recipe())
    assert (tmp_path / "seed.json").exists()
    on_disk = read_sidecar(tmp_path / "seed.json")
    assert on_disk["recipe_id"] == recipe_id(_gen_recipe()) == side["recipe_id"]
    assert on_disk["output"]["channels"] == 1 and on_disk["output"]["notes"] > 5
    assert on_disk["metrics"]["periodicity"] > 0 and on_disk["environment"]["numpy"] == np.__version__
    res = verify(tmp_path / "seed.json")
    assert res["file_matches_sidecar"] and res["rebuild_matches_sidecar"] and res["same_numpy"]


def test_verify_detects_edited_wav_and_edited_recipe(tmp_path):
    wav = tmp_path / "seed.wav"
    write_seed(wav, _gen_recipe())
    raw = bytearray(wav.read_bytes()); raw[-200] ^= 0x7F          # corrupt one audio byte
    wav.write_bytes(bytes(raw))
    res = verify(tmp_path / "seed.json")
    assert res["file_matches_sidecar"] is False and res["rebuild_matches_sidecar"] is True
    write_seed(wav, _gen_recipe())
    j = json.loads((tmp_path / "seed.json").read_text())
    j["recipe"]["config"]["strength"] = 0.9                        # recipe no longer describes the file
    (tmp_path / "seed.json").write_text(json.dumps(j))
    assert verify(tmp_path / "seed.json")["rebuild_matches_sidecar"] is False


def test_file_inputs_are_hash_checked_and_paths_are_relative(tmp_path):
    S, f, fs = drawn_spectrogram(seed=1)
    np.save(tmp_path / "art.npy", S)
    notes = make_melody("motif", 0.3, 1)
    (tmp_path / "n.json").write_text(notes_to_json(notes))
    out = tmp_path / "sub" / "seed.wav"
    out.parent.mkdir()
    base = out.parent
    recipe = {"input": {"kind": "file", **file_ref(tmp_path / "art.npy", base), "fmax": 8000.0, "frame_sec": 0.032},
              "melody": {"kind": "notes", **file_ref(tmp_path / "n.json", base)},
              "config": default_config(seed=1), "gs": None, "channels": 2}
    assert recipe["input"]["path"] == "../art.npy" or recipe["input"]["path"].endswith("art.npy")
    assert "\\" not in recipe["input"]["path"]
    write_seed(out, recipe)
    assert verify(tmp_path / "sub" / "seed.json")["rebuild_matches_sidecar"]
    (tmp_path / "n.json").write_text(notes_to_json(notes[:-1]))     # notes edited after the fact
    with pytest.raises(ValueError, match="changed since the recipe"):
        build_seed(recipe, base)
    (tmp_path / "n.json").unlink()
    with pytest.raises(FileNotFoundError):
        build_seed(recipe, base)


def test_native_recipe_has_no_metrics_but_still_verifies(tmp_path):
    r = _gen_recipe(); r["melody"] = {"kind": "native"}
    side = write_seed(tmp_path / "t.wav", r)
    assert side["metrics"] is None and side["output"]["notes"] is None
    assert verify(tmp_path / "t.json")["rebuild_matches_sidecar"]
