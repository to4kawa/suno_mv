"""Standard MIDI File (format 0) writer, standard library only.

Notes are in seconds; they are placed on a tick grid at `bpm` so the DAW / Suno shows the
real tempo. Intended for monophonic melodies (the makers here never overlap same-pitch notes).
"""
from __future__ import annotations

import pathlib
import struct


def _vlq(n: int) -> bytes:
    out = [n & 0x7F]
    n >>= 7
    while n:
        out.append((n & 0x7F) | 0x80)
        n >>= 7
    return bytes(reversed(out))


def notes_to_midi_bytes(notes, bpm=120.0, ppq=480, name="idm_seed melody", program=0, channel=0):
    ticks_per_sec = ppq * bpm / 60.0
    ev = []  # (tick, order, bytes); note-off sorts before note-on on the same tick
    for n in notes:
        p = int(round(n.midi))
        on = int(round(n.start * ticks_per_sec))
        off = max(on + 1, int(round((n.start + n.dur) * ticks_per_sec)))
        vel = max(1, min(127, int(round(n.vel * 127))))
        ev.append((on, 1, bytes([0x90 | channel, p, vel])))
        ev.append((off, 0, bytes([0x80 | channel, p, 0])))
    ev.sort(key=lambda e: (e[0], e[1]))
    nm = name.encode("ascii", "replace")
    trk = bytearray()
    trk += b"\x00\xff\x03" + _vlq(len(nm)) + nm
    trk += b"\x00\xff\x51\x03" + int(round(60_000_000 / bpm)).to_bytes(3, "big")
    trk += b"\x00\xff\x58\x04\x04\x02\x18\x08"                  # 4/4
    trk += b"\x00" + bytes([0xC0 | channel, program])
    prev = 0
    for tick, _o, data in ev:
        trk += _vlq(tick - prev) + data
        prev = tick
    trk += b"\x00\xff\x2f\x00"
    return (b"MThd" + struct.pack(">IHHH", 6, 0, 1, ppq)
            + b"MTrk" + struct.pack(">I", len(trk)) + bytes(trk))


def write_midi(path, notes, bpm=120.0, **kw):
    pathlib.Path(path).write_bytes(notes_to_midi_bytes(notes, bpm, **kw))
