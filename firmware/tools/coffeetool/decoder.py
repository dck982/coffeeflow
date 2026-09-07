"""Décodeur de trames — lit un RawFrame déjà désassemblé (peu importe le
transport, voir transport.py) et produit une ligne de texte lisible,
horodatée. C'est le cœur de la phase 1 : "l'outil décode ... sans matériel"
(docs/firmware-implementation.md).
"""

from __future__ import annotations

import datetime

from . import messages
from .logcodes import log_code_name, log_severity_name
from .protocol import Dest, MessageType, Node, decode_can_id
from .framing import RawFrame

_NODE_NAMES = {Node.SCREEN: "écran", Node.SENSORS: "capteurs"}
_DEST_NAMES = {Dest.BROADCAST: "broadcast", Dest.SCREEN: "écran", Dest.SENSORS: "capteurs"}


def _node_name(value) -> str:
    return _NODE_NAMES.get(value, f"?{value}")


def _dest_name(value) -> str:
    return _DEST_NAMES.get(value, f"?{value}")


def _format_payload(message_type: MessageType, data: bytes) -> str:
    payload_cls = messages.PAYLOAD_BY_TYPE.get(message_type)
    if payload_cls is None:
        return f"data={data.hex()}" if data else ""
    try:
        payload = payload_cls.unpack(data)
    except ValueError as exc:
        return f"(charge utile invalide: {exc})"

    if message_type is MessageType.LOG:
        p: messages.LogPayload = payload
        extra = []
        if p.arg16:
            extra.append(f"arg16={p.arg16}")
        if p.arg32:
            extra.append(f"arg32={p.arg32}")
        extra_str = " " + " ".join(extra) if extra else ""
        return f"{log_code_name(p.code)} [{log_severity_name(p.severity)}]{extra_str}"

    if message_type is MessageType.PONG:
        p = payload
        return f"node={_node_name(p.node)} v{p.version_major}.{p.version_minor}.{p.version_patch} uptime={p.uptime_s}s"

    if message_type is MessageType.SET:
        p = payload
        parts = []
        if p.set_ssr:
            parts.append(f"ssr={'on' if p.ssr else 'off'}")
        if p.set_dimmer:
            parts.append(f"dimmer={p.dimmer}%")
        parts.append(f"ttl={p.ttl_ms or 500}ms")
        return " ".join(parts)

    if message_type is MessageType.REQSTATUS:
        p = payload
        target = p.target_type.name if isinstance(p.target_type, MessageType) else hex(p.target_type)
        return f"cible={target} periode={p.period_ms}ms" if p.period_ms else f"cible={target} arrêt"

    if message_type is MessageType.STATUS_PRESSURE:
        p = payload
        return f"pression_brute={p.pressure_raw} temp_brute={p.temperature_raw} t={p.timestamp_ms}ms flags={p.flags:#04b}"

    if message_type is MessageType.STATUS_FLOW:
        p = payload
        return f"impulsions={p.pulse_count} dernier_front={p.last_edge_ms}ms flags={p.flags:#04b}"

    if message_type is MessageType.STATUS_ACTUATORS:
        p = payload
        return (
            f"ssr={'on' if p.ssr else 'off'} dimmer={p.dimmer}% "
            f"bail_restant={p.lease_remaining_ms}ms marche_continue={p.continuous_on_ms}ms flags={p.flags:#05b}"
        )

    if message_type is MessageType.FLASH_CTRL:
        p = payload
        if p.subcmd == messages.FlashSubCmd.BEGIN:
            return f"BEGIN taille={p.image_size}"
        if p.subcmd == messages.FlashSubCmd.BLOCK_ACK:
            return f"BLOCK_ACK bloc={p.block_number} crc16={p.block_crc16:#06x}"
        if p.subcmd == messages.FlashSubCmd.END:
            return f"END crc32={p.image_crc32:#010x}"
        return "ABORT"

    return f"data={data.hex()}"


def format_timestamp(ts: float) -> str:
    return datetime.datetime.fromtimestamp(ts).strftime("%H:%M:%S.%f")[:-3]


def decode_frame(ts: float, frame: RawFrame) -> str:
    can_id = decode_can_id(frame.can_id)
    type_name = can_id.type.name if isinstance(can_id.type, MessageType) else f"0x{frame.can_id:03x}(inconnu)"
    route = f"{_node_name(can_id.src)}→{_dest_name(can_id.dest)}"
    detail = _format_payload(can_id.type, frame.data) if isinstance(can_id.type, MessageType) else frame.data.hex()
    line = f"[{format_timestamp(ts)}] {route:<18} {type_name:<17}"
    if detail:
        line += f" {detail}"
    return line
