"""Cadrage série (USB CDC) — portage à la main de
firmware/common/include/common/framing.hpp. Voir docs/firmware.md et
docs/firmware-implementation.md, phase 1.

Le WebSocket transporte le PDU tel quel, un message = une trame ; seul le
fil série a besoin d'un cadrage, parce qu'un flux d'octets n'a pas de
bordure.

PDU (identique sur les deux transports), little-endian :
  [0..1]  id     uint16 — bits 10..0 = identifiant CAN, bits 15..11 = 0
  [2]     dlc    0..8
  [3..]   data   dlc octets
  [fin]   crc16  uint16, CRC-16-CCITT (poly 0x1021, init 0xFFFF) sur id..data

Sur le fil série : COBS(PDU) puis un octet 0x00 de fin de trame.
"""

from __future__ import annotations

import struct
from collections import deque
from dataclasses import dataclass, field

from .crc import crc16_ccitt

MAX_PDU_SIZE = 2 + 1 + 8 + 2  # id + dlc + data + crc16 = 13
MAX_COBS_SIZE = MAX_PDU_SIZE + (MAX_PDU_SIZE // 254) + 1


@dataclass
class RawFrame:
    can_id: int  # 11 bits significatifs
    data: bytes = b""

    @property
    def dlc(self) -> int:
        return len(self.data)


def encode_pdu(frame: RawFrame) -> bytes:
    if not (0 <= frame.can_id <= 0x7FF):
        raise ValueError(f"can_id hors 11 bits: {frame.can_id:#x}")
    if len(frame.data) > 8:
        raise ValueError(f"dlc > 8: {len(frame.data)}")
    body = struct.pack("<HB", frame.can_id, len(frame.data)) + frame.data
    crc = crc16_ccitt(body)
    return body + struct.pack("<H", crc)


def decode_pdu(pdu: bytes) -> RawFrame | None:
    if len(pdu) < 5:
        return None
    can_id, dlc = struct.unpack("<HB", pdu[:3])
    if dlc > 8 or len(pdu) != 3 + dlc + 2:
        return None
    body, crc_bytes = pdu[: 3 + dlc], pdu[3 + dlc :]
    if crc16_ccitt(body) != struct.unpack("<H", crc_bytes)[0]:
        return None
    if can_id > 0x7FF:
        return None
    return RawFrame(can_id=can_id, data=body[3:])


def cobs_encode(data: bytes) -> bytes:
    """Référence : Cheshire & Baker, "Consistent Overhead Byte Stuffing"."""
    out = bytearray()
    code_index = 0
    out.append(0)  # code provisoire, patché plus bas
    code = 1
    for byte in data:
        if byte == 0:
            out[code_index] = code
            code = 1
            code_index = len(out)
            out.append(0)
        else:
            out.append(byte)
            code += 1
            if code == 0xFF:
                out[code_index] = code
                code = 1
                code_index = len(out)
                out.append(0)
    out[code_index] = code
    return bytes(out)


def cobs_decode(data: bytes) -> bytes | None:
    out = bytearray()
    read_index = 0
    length = len(data)
    while read_index < length:
        code = data[read_index]
        if code == 0:
            return None  # un 0x00 ne peut apparaître qu'en fin de trame
        if read_index + code > length and code != 1:
            return None
        read_index += 1
        for _ in range(1, code):
            if read_index >= length:
                return None
            out.append(data[read_index])
            read_index += 1
        if code != 0xFF and read_index != length:
            out.append(0)
    return bytes(out)


def encode_framed(frame: RawFrame) -> bytes:
    return cobs_encode(encode_pdu(frame)) + b"\x00"


class StreamDecoder:
    """Décodeur incrémental pour un flux d'octets sans bordure (USB CDC).

    Contrairement au C++ (callback), côté Mac on empile les trames décodées
    dans une file et on les dépile à la demande — plus naturel pour un
    Transport qui expose recv().
    """

    def __init__(self) -> None:
        self._buffer = bytearray()
        self._frames: deque[RawFrame] = deque()
        self.dropped_count = 0

    def push_byte(self, byte: int) -> None:
        if byte == 0x00:
            if not self._buffer:
                return  # 0x00 double, keepalive optionnel
            pdu = cobs_decode(bytes(self._buffer))
            frame = decode_pdu(pdu) if pdu is not None else None
            if frame is not None:
                self._frames.append(frame)
            else:
                self.dropped_count += 1
            self._buffer.clear()
            return
        if len(self._buffer) >= MAX_COBS_SIZE:
            self.dropped_count += 1
            self._buffer.clear()
            return
        self._buffer.append(byte)

    def push_bytes(self, data: bytes) -> None:
        for byte in data:
            self.push_byte(byte)

    def pop_frame(self) -> RawFrame | None:
        return self._frames.popleft() if self._frames else None
