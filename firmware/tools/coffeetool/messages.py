"""Charges utiles du protocole — portage à la main de
firmware/common/include/common/messages.hpp. Voir docs/firmware.md, section
"Charges utiles". `pack()` renvoie exactement 8 octets ; `unpack()` lève
ValueError si le buffer est trop court, plutôt que d'échouer silencieusement
comme côté C++ (bool de retour) — ce module ne tourne jamais sous
contrainte temps réel.

Endianness : little-endian pour tous les champs multi-octets, comme fixé
dans docs/firmware.md ("Décisions déjà prises").
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from enum import IntEnum

from .protocol import MessageType, Node


def _need(data: bytes, n: int, what: str) -> None:
    if len(data) < n:
        raise ValueError(f"{what}: {len(data)} octet(s), {n} attendu(s) au moins")


@dataclass
class SetPayload:
    set_ssr: bool = False
    set_dimmer: bool = False
    ssr: bool = False
    dimmer: int = 0
    ttl_ms: int = 0

    def pack(self) -> bytes:
        mask = (0x01 if self.set_ssr else 0) | (0x02 if self.set_dimmer else 0)
        return struct.pack("<BBB H 3x", mask, 1 if self.ssr else 0, self.dimmer, self.ttl_ms)

    @staticmethod
    def unpack(data: bytes) -> "SetPayload":
        _need(data, 5, "SET")
        mask = data[0]
        (ttl_ms,) = struct.unpack("<H", data[3:5])
        return SetPayload(
            set_ssr=bool(mask & 0x01),
            set_dimmer=bool(mask & 0x02),
            ssr=data[1] != 0,
            dimmer=data[2],
            ttl_ms=ttl_ms,
        )


@dataclass
class PongPayload:
    node: Node = Node.SENSORS
    version_major: int = 0
    version_minor: int = 0
    version_patch: int = 0
    uptime_s: int = 0

    def pack(self) -> bytes:
        return struct.pack(
            "<BBBB I",
            int(self.node),
            self.version_major,
            self.version_minor,
            self.version_patch,
            self.uptime_s,
        )

    @staticmethod
    def unpack(data: bytes) -> "PongPayload":
        _need(data, 8, "PONG")
        node_raw, major, minor, patch, uptime_s = struct.unpack("<BBBBI", data[:8])
        node = Node(node_raw) if node_raw in (Node.SCREEN, Node.SENSORS) else node_raw
        return PongPayload(node, major, minor, patch, uptime_s)


@dataclass
class ReqStatusPayload:
    target_type: MessageType = MessageType.STATUS_PRESSURE
    period_ms: int = 0

    def pack(self) -> bytes:
        return struct.pack("<BH 5x", int(self.target_type), self.period_ms)

    @staticmethod
    def unpack(data: bytes) -> "ReqStatusPayload":
        _need(data, 3, "REQSTATUS")
        target_raw = data[0]
        (period_ms,) = struct.unpack("<H", data[1:3])
        target = MessageType(target_raw) if target_raw in iter(MessageType) else target_raw
        return ReqStatusPayload(target, period_ms)


@dataclass
class StatusPressurePayload:
    pressure_raw: int = 0      # 24 bits utiles
    temperature_raw: int = 0   # 16 bits
    timestamp_ms: int = 0      # 16 bits bas
    flags: int = 0

    def pack(self) -> bytes:
        p = self.pressure_raw & 0xFFFFFF
        return bytes([p & 0xFF, (p >> 8) & 0xFF, (p >> 16) & 0xFF]) + struct.pack(
            "<HH", self.temperature_raw, self.timestamp_ms
        ) + bytes([self.flags])

    @staticmethod
    def unpack(data: bytes) -> "StatusPressurePayload":
        _need(data, 8, "STATUS_PRESSURE")
        pressure_raw = data[0] | (data[1] << 8) | (data[2] << 16)
        temperature_raw, timestamp_ms = struct.unpack("<HH", data[3:7])
        return StatusPressurePayload(pressure_raw, temperature_raw, timestamp_ms, data[7])


@dataclass
class StatusFlowPayload:
    pulse_count: int = 0
    last_edge_ms: int = 0
    flags: int = 0

    def pack(self) -> bytes:
        return struct.pack("<IH", self.pulse_count, self.last_edge_ms) + bytes([self.flags, 0])

    @staticmethod
    def unpack(data: bytes) -> "StatusFlowPayload":
        _need(data, 7, "STATUS_FLOW")
        pulse_count, last_edge_ms = struct.unpack("<IH", data[:6])
        return StatusFlowPayload(pulse_count, last_edge_ms, data[6])


@dataclass
class StatusActuatorsPayload:
    ssr: bool = False
    dimmer: int = 0
    lease_remaining_ms: int = 0
    continuous_on_ms: int = 0
    flags: int = 0

    def pack(self) -> bytes:
        return struct.pack(
            "<BBHH B x",
            1 if self.ssr else 0,
            self.dimmer,
            self.lease_remaining_ms,
            self.continuous_on_ms,
            self.flags,
        )

    @staticmethod
    def unpack(data: bytes) -> "StatusActuatorsPayload":
        _need(data, 7, "STATUS_ACTUATORS")
        ssr = data[0] != 0
        dimmer = data[1]
        lease_remaining_ms, continuous_on_ms = struct.unpack("<HH", data[2:6])
        return StatusActuatorsPayload(ssr, dimmer, lease_remaining_ms, continuous_on_ms, data[6])


@dataclass
class LogPayload:
    code: int = 0
    severity: int = 0
    arg16: int = 0
    arg32: int = 0

    def pack(self) -> bytes:
        return struct.pack("<BBHI", self.code, self.severity, self.arg16, self.arg32)

    @staticmethod
    def unpack(data: bytes) -> "LogPayload":
        _need(data, 8, "LOG")
        code, severity, arg16, arg32 = struct.unpack("<BBHI", data[:8])
        return LogPayload(code, severity, arg16, arg32)


class FlashSubCmd(IntEnum):
    BEGIN = 0
    BLOCK_ACK = 1
    END = 2
    ABORT = 3


@dataclass
class FlashCtrlPayload:
    """Layout provisoire — voir messages.hpp, à revalider en phase 4."""

    subcmd: FlashSubCmd = FlashSubCmd.ABORT
    image_size: int = 0    # BEGIN
    block_number: int = 0  # BLOCK_ACK
    block_crc16: int = 0   # BLOCK_ACK
    image_crc32: int = 0   # END

    def pack(self) -> bytes:
        head = bytes([int(self.subcmd)])
        if self.subcmd == FlashSubCmd.BEGIN:
            return head + struct.pack("<I", self.image_size) + b"\x00\x00\x00"
        if self.subcmd == FlashSubCmd.BLOCK_ACK:
            return head + struct.pack("<HH", self.block_number, self.block_crc16) + b"\x00\x00\x00"
        if self.subcmd == FlashSubCmd.END:
            return head + struct.pack("<I", self.image_crc32) + b"\x00\x00\x00"
        return head + b"\x00" * 7  # ABORT

    @staticmethod
    def unpack(data: bytes) -> "FlashCtrlPayload":
        _need(data, 1, "FLASH_CTRL")
        subcmd = FlashSubCmd(data[0])
        if subcmd == FlashSubCmd.BEGIN:
            _need(data, 5, "FLASH_CTRL BEGIN")
            (image_size,) = struct.unpack("<I", data[1:5])
            return FlashCtrlPayload(subcmd, image_size=image_size)
        if subcmd == FlashSubCmd.BLOCK_ACK:
            _need(data, 5, "FLASH_CTRL BLOCK_ACK")
            block_number, block_crc16 = struct.unpack("<HH", data[1:5])
            return FlashCtrlPayload(subcmd, block_number=block_number, block_crc16=block_crc16)
        if subcmd == FlashSubCmd.END:
            _need(data, 5, "FLASH_CTRL END")
            (image_crc32,) = struct.unpack("<I", data[1:5])
            return FlashCtrlPayload(subcmd, image_crc32=image_crc32)
        return FlashCtrlPayload(subcmd)


# FLASH_DATA (0x39) — 8 octets bruts, aucun en-tête : la trame CAN EST la
# charge utile. Rien à empaqueter.

PAYLOAD_BY_TYPE = {
    MessageType.SET: SetPayload,
    MessageType.PONG: PongPayload,
    MessageType.REQSTATUS: ReqStatusPayload,
    MessageType.STATUS_PRESSURE: StatusPressurePayload,
    MessageType.STATUS_FLOW: StatusFlowPayload,
    MessageType.STATUS_ACTUATORS: StatusActuatorsPayload,
    MessageType.LOG: LogPayload,
    MessageType.FLASH_CTRL: FlashCtrlPayload,
}
