"""Identifiant CAN 11 bits — portage à la main de
firmware/common/include/common/protocol.hpp. Voir docs/firmware.md,
section "Protocole CAN".

  bits 10..5   type   (64 valeurs) — valeur basse = priorité bus haute
  bits  4..3   dest   (0 broadcast, 1 écran, 2 capteurs)
  bits  2..0   src    (1 écran, 2 capteurs)
"""

from dataclasses import dataclass
from enum import IntEnum


class Node(IntEnum):
    SCREEN = 1
    SENSORS = 2


class Dest(IntEnum):
    BROADCAST = 0
    SCREEN = 1
    SENSORS = 2


class MessageType(IntEnum):
    STOP = 0x00
    SET = 0x01
    RESET = 0x02
    PING = 0x08
    PONG = 0x09
    REQSTATUS = 0x10
    STATUS_PRESSURE = 0x20
    STATUS_FLOW = 0x21
    STATUS_ACTUATORS = 0x22
    LOG = 0x30
    FLASH_CTRL = 0x38
    FLASH_DATA = 0x39


def is_known_message_type(value: int) -> bool:
    try:
        MessageType(value)
        return True
    except ValueError:
        return False


@dataclass(frozen=True)
class CanId:
    type: MessageType
    dest: Dest
    src: Node


def encode_can_id(can_id: CanId) -> int:
    type_bits = int(can_id.type) & 0x3F
    dest_bits = int(can_id.dest) & 0x03
    src_bits = int(can_id.src) & 0x07
    return (type_bits << 5) | (dest_bits << 3) | src_bits


def decode_can_id(raw: int) -> CanId:
    type_bits = (raw >> 5) & 0x3F
    dest_bits = (raw >> 3) & 0x03
    src_bits = raw & 0x07
    message_type = MessageType(type_bits) if is_known_message_type(type_bits) else None
    dest = Dest(dest_bits) if dest_bits in (Dest.BROADCAST, Dest.SCREEN, Dest.SENSORS) else dest_bits
    src = Node(src_bits) if src_bits in (Node.SCREEN, Node.SENSORS) else src_bits
    return CanId(type=message_type, dest=dest, src=src)
