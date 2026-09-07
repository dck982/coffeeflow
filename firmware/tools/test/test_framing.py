from coffeetool.framing import (
    RawFrame,
    StreamDecoder,
    cobs_decode,
    cobs_encode,
    decode_pdu,
    encode_framed,
    encode_pdu,
)
from coffeetool.protocol import CanId, Dest, MessageType, Node, encode_can_id


def test_pdu_roundtrip():
    frame = RawFrame(
        can_id=encode_can_id(CanId(MessageType.PONG, Dest.SCREEN, Node.SENSORS)),
        data=bytes(range(0x10, 0x18)),
    )
    pdu = encode_pdu(frame)
    assert len(pdu) == 3 + frame.dlc + 2
    out = decode_pdu(pdu)
    assert out == frame


def test_pdu_rejects_bad_crc():
    frame = RawFrame(can_id=encode_can_id(CanId(MessageType.PING, Dest.BROADCAST, Node.SCREEN)))
    pdu = bytearray(encode_pdu(frame))
    pdu[-1] ^= 0xFF
    assert decode_pdu(bytes(pdu)) is None


def test_cobs_roundtrip_with_zeros():
    data = bytes([0] * 13)
    data = data[:1] + bytes([0x05]) + data[2:]  # un seul octet non nul au milieu
    encoded = cobs_encode(data)
    assert 0 not in encoded
    assert cobs_decode(encoded) == data


def test_encode_framed_roundtrip_via_stream_decoder():
    frame = RawFrame(
        can_id=encode_can_id(CanId(MessageType.SET, Dest.SENSORS, Node.SCREEN)),
        data=bytes([0x03, 0x01, 42, 0xF4, 0x01, 0, 0, 0]),
    )
    framed = encode_framed(frame)
    assert framed[-1] == 0x00

    decoder = StreamDecoder()
    decoder.push_bytes(framed)
    received = decoder.pop_frame()
    assert received == frame
    assert decoder.pop_frame() is None
    assert decoder.dropped_count == 0


def test_stream_decoder_resyncs_after_garbage():
    frame = RawFrame(can_id=encode_can_id(CanId(MessageType.PING, Dest.BROADCAST, Node.SENSORS)))
    framed = encode_framed(frame)

    decoder = StreamDecoder()
    decoder.push_bytes(bytes([0x7A, 0x11, 0x00]))  # trame invalide, close par 0x00
    assert decoder.pop_frame() is None
    assert decoder.dropped_count == 1

    decoder.push_bytes(framed)
    assert decoder.pop_frame() == frame


def test_golden_vectors_from_cpp():
    """Vérifie l'octet-à-octet contre firmware/common (voir framing.hpp) :
    ces deux trames ont été produites en exécutant encode_framed() côté
    C++ pour garantir que le portage Python n'a pas dérivé."""
    ping = RawFrame(can_id=encode_can_id(CanId(MessageType.PING, Dest.BROADCAST, Node.SCREEN)))
    assert encode_framed(ping).hex() == "030101039dc800"

    set_frame = RawFrame(
        can_id=encode_can_id(CanId(MessageType.SET, Dest.SENSORS, Node.SCREEN)),
        data=bytes([0x03, 0x01, 42, 0xF4, 0x01]),
    )
    assert encode_framed(set_frame).hex() == "0231090503012af401756500"
