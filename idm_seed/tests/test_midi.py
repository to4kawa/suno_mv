import struct

from idm_seed.melody import make_classic_melody, make_motif_melody
from idm_seed.midi import notes_to_midi_bytes


def _vlq_read(b, i):
    v = 0
    while True:
        c = b[i]; i += 1
        v = (v << 7) | (c & 0x7F)
        if not c & 0x80:
            return v, i


def parse(data):
    """Minimal independent SMF reader: returns (ppq, tempo_us, notes[(on_tick, off_tick, pitch, vel)])."""
    assert data[:4] == b"MThd" and struct.unpack(">IHH", data[4:12]) == (6, 0, 1)
    ppq = struct.unpack(">H", data[12:14])[0]
    assert data[14:18] == b"MTrk"
    ln = struct.unpack(">I", data[18:22])[0]
    trk, i, t, tempo, open_, notes, ended = data[22:22 + ln], 0, 0, None, {}, [], False
    assert len(data) == 22 + ln
    while i < len(trk):
        d, i = _vlq_read(trk, i); t += d
        st = trk[i]; i += 1
        if st == 0xFF:
            typ = trk[i]; i += 1
            l, i = _vlq_read(trk, i)
            body = trk[i:i + l]; i += l
            if typ == 0x51:
                tempo = int.from_bytes(body, "big")
            if typ == 0x2F:
                ended = True
        elif st & 0xF0 == 0xC0:
            i += 1
        elif st & 0xF0 == 0x90:
            open_[trk[i]] = (t, trk[i + 1]); i += 2
        elif st & 0xF0 == 0x80:
            on, vel = open_.pop(trk[i]); notes.append((on, t, trk[i], vel)); i += 2
        else:
            raise AssertionError(hex(st))
    assert ended and not open_
    return ppq, tempo, notes


def _roundtrip(notes, bpm):
    ppq, tempo, got = parse(notes_to_midi_bytes(notes, bpm))
    assert tempo == round(60_000_000 / bpm)
    got.sort()
    assert len(got) == len(notes)
    tps = ppq * bpm / 60.0
    for (on, off, p, vel), n in zip(got, notes):
        assert p == round(n.midi)
        assert abs(on - n.start * tps) <= 0.5 + 1e-9
        assert abs((off - on) - n.dur * tps) <= 1.0 + 1e-9
        assert 1 <= vel <= 127


def test_motif_melody_roundtrips_at_its_tempo():
    _roundtrip(make_motif_melody(2, 3, bpm=96, scale="dorian", root=62), 96)


def test_classic_melody_roundtrips_at_120():
    _roundtrip(make_classic_melody(2, 1), 120)


def test_no_overlap_and_abutting_notes_do_not_stick():
    notes = make_motif_melody(1, 2)
    _, _, got = parse(notes_to_midi_bytes(notes, 120))   # parse() asserts every note-on gets its off
    got.sort()
    assert all(a[1] <= b[0] for a, b in zip(got, got[1:]))
