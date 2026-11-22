"""idm_seed: turn a drawn spectrogram into an IDM-style timbre seed (WAV) for Suno."""
from .core import (GlitchPlan, HarmonicStructure, apply_partial_correction, cosine_similarity,
                   detect_harmonic_structure, make_glitch_plan)
from .pipeline import SeedBuilder, SeedConfig
from .render import Note, Timbre, midi_to_hz, normalize_peak, write_wav
