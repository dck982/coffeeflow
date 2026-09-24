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
_BOILER_ADC_STAGES = {
    0: "bus indisponible", 1: "ajout du périphérique", 2: "configuration A0", 3: "statut A0", 4: "résultat A0",
    5: "configuration A1", 6: "statut A1", 7: "résultat A1",
}


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
        name = log_code_name(p.code)
        severity = log_severity_name(p.severity)
        if name == "BOILER_ADC_NOT_FOUND":
            return f"{name} [{severity}] adresses=0x48–0x4b dernier_esp_err={p.arg32:#x}"
        if name == "BOILER_ADC_I2C_ERROR":
            stage = _BOILER_ADC_STAGES.get(p.arg16 >> 8, f"étape {p.arg16 >> 8}")
            return f"{name} [{severity}] adresse={p.arg16 & 0xff:#04x} étape={stage} esp_err={p.arg32:#x}"
        if name == "BOILER_ADC_CONVERSION_TIMEOUT":
            return f"{name} [{severity}] adresse={p.arg16 & 0xff:#04x} canal=A{p.arg16 >> 8}"
        if name == "BOILER_NTC_INVALID_READING":
            a0 = p.arg32 >> 16
            a1 = p.arg32 & 0xffff
            if a0 & 0x8000:
                a0 -= 0x10000
            if a1 & 0x8000:
                a1 -= 0x10000
            return (f"{name} [{severity}] adresse={p.arg16:#04x} "
                    f"A0_brut={a0} A1_brut={a1}")
        if name == "BOILER_ADC_RECOVERED":
            return f"{name} [{severity}] adresse={p.arg16:#04x} interruption={p.arg32}ms"
        extra = []
        if p.arg16:
            extra.append(f"arg16={p.arg16}")
        if p.arg32:
            extra.append(f"arg32={p.arg32}")
        extra_str = " " + " ".join(extra) if extra else ""
        return f"{name} [{severity}]{extra_str}"

    if message_type is MessageType.PING and not data:
        return "(PING ancien, sans identité)"

    if message_type in (MessageType.PING, MessageType.PONG):
        p = payload
        return f"node={_node_name(p.node)} v{p.version_major}.{p.version_minor}.{p.version_patch} uptime={p.uptime_s}s"

    if message_type is MessageType.SET:
        p = payload
        return f"pompe={p.dimmer}% ttl={p.ttl_ms or 500}ms"

    if message_type is MessageType.SET_HEATING:
        p = payload
        return f"chauffage={'on' if p.on else 'off'} duree={p.duration_ms}ms"

    if message_type is MessageType.SET_HEATING_POWER:
        p = payload
        return f"puissance_chaudiere={p.power_permille / 10:.1f}% bail={p.lease_ms}ms"

    if message_type is MessageType.CONFIRM_SENSORS_OTA:
        return f"version_protocole_chauffage={payload.protocol_patch}"

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
            f"vanne={'ouverte' if p.ssr else 'fermee'} pompe={p.dimmer}% "
            f"bail_restant={p.lease_remaining_ms}ms marche_continue={p.continuous_on_ms}ms flags={p.flags:#05b}"
        )

    if message_type is MessageType.STATUS_HEATING:
        p = payload
        return (f"chauffage={'on' if p.heater_on else 'off'} bail_restant={p.lease_remaining_ms}ms "
                f"puissance={p.power_permille / 10:.1f}% capable={'fin' if p.fine_power_capable else 'diagnostic'}")

    if message_type is MessageType.FLASH_CTRL:
        p = payload
        if p.subcmd == messages.FlashSubCmd.BEGIN:
            return f"BEGIN taille={p.image_size}"
        if p.subcmd == messages.FlashSubCmd.BLOCK_ACK:
            return f"BLOCK_ACK bloc={p.block_number} crc16={p.block_crc16:#06x}"
        if p.subcmd == messages.FlashSubCmd.BLOCK_START:
            return f"BLOCK_START bloc={p.block_number} crc16={p.block_crc16:#06x}"
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
