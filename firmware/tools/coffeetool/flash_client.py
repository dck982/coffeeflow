"""Client de flash (BEGIN, blocs, END) — docs/firmware-implementation.md,
phase 1 : "Le client de flash vit ici aussi". Layout FLASH_CTRL provisoire,
voir messages.hpp / docs/firmware.md ("Ce qui reste à trancher") — à
revalider pour de vrai en phase 4, contre du matériel.

Séquence (docs/firmware.md, "Flash — le seul cas de réassemblage") :
  1. FLASH_CTRL BEGIN porte la taille.
  2. FLASH_DATA : 8 octets de données pures, aucun en-tête.
  3. Tous les 2 ko (256 trames), on attend un FLASH_CTRL BLOCK_ACK avec le
     numéro de bloc et le CRC16 : c'est le contrôle de flux.
  4. Bloc faux → on rejoue le bloc, pas l'image entière.
  5. FLASH_CTRL END porte le CRC32 global.
"""

from __future__ import annotations

from dataclasses import dataclass

from .crc import crc16_ccitt, crc32_ieee
from .framing import RawFrame
from .messages import FlashCtrlPayload, FlashSubCmd
from .protocol import CanId, Dest, MessageType, Node, encode_can_id
from .transport import Transport

BLOCK_SIZE = 2048
MAX_RETRIES_PER_BLOCK = 3
ACK_TIMEOUT_S = 5.0


class FlashError(RuntimeError):
    pass


@dataclass
class FlashProgress:
    block_number: int
    block_count: int
    bytes_sent: int
    image_size: int


def _flash_ctrl_id(src: Node, dest: Dest) -> int:
    return encode_can_id(CanId(MessageType.FLASH_CTRL, dest, src))


def _flash_data_id(src: Node, dest: Dest) -> int:
    return encode_can_id(CanId(MessageType.FLASH_DATA, dest, src))


def _wait_flash_ctrl(transport: Transport, timeout: float) -> FlashCtrlPayload | None:
    result = transport.recv(timeout=timeout)
    if result is None:
        return None
    _, frame = result
    can_id = frame.can_id
    if (can_id >> 5) & 0x3F != int(MessageType.FLASH_CTRL):
        return None
    try:
        return FlashCtrlPayload.unpack(frame.data)
    except ValueError:
        return None


def flash(
    transport: Transport,
    image: bytes,
    *,
    src: Node = Node.SCREEN,
    dest: Dest = Dest.SENSORS,
    on_progress=None,
) -> None:
    """Envoie `image` par le protocole FLASH_CTRL/FLASH_DATA. Lève
    FlashError si un bloc échoue après MAX_RETRIES_PER_BLOCK essais, ou si
    l'acquittement final n'arrive pas."""

    ctrl_id = _flash_ctrl_id(src, dest)
    data_id = _flash_data_id(src, dest)

    begin = FlashCtrlPayload(subcmd=FlashSubCmd.BEGIN, image_size=len(image))
    transport.send(RawFrame(ctrl_id, begin.pack()))

    ack = _wait_flash_ctrl(transport, ACK_TIMEOUT_S)
    if ack is None or ack.subcmd != FlashSubCmd.BLOCK_ACK or ack.block_number != 0:
        raise FlashError("pas d'acquittement de BEGIN (effacement de partition)")

    block_count = (len(image) + BLOCK_SIZE - 1) // BLOCK_SIZE
    for block_number in range(block_count):
        block = image[block_number * BLOCK_SIZE : (block_number + 1) * BLOCK_SIZE]
        expected_crc = crc16_ccitt(block)

        for attempt in range(1, MAX_RETRIES_PER_BLOCK + 1):
            for offset in range(0, len(block), 8):
                transport.send(RawFrame(data_id, block[offset : offset + 8]))

            ack = _wait_flash_ctrl(transport, ACK_TIMEOUT_S)
            if (
                ack is not None
                and ack.subcmd == FlashSubCmd.BLOCK_ACK
                and ack.block_number == block_number + 1
                and ack.block_crc16 == expected_crc
            ):
                break
            if attempt == MAX_RETRIES_PER_BLOCK:
                raise FlashError(f"bloc {block_number} refusé après {attempt} essais")
        else:
            raise FlashError(f"bloc {block_number}: pas d'acquittement")

        if on_progress is not None:
            on_progress(
                FlashProgress(block_number + 1, block_count, min((block_number + 1) * BLOCK_SIZE, len(image)), len(image))
            )

    end = FlashCtrlPayload(subcmd=FlashSubCmd.END, image_crc32=crc32_ieee(image))
    transport.send(RawFrame(ctrl_id, end.pack()))

    final_ack = _wait_flash_ctrl(transport, ACK_TIMEOUT_S)
    if final_ack is None or final_ack.subcmd != FlashSubCmd.END:
        raise FlashError("pas de confirmation finale — surveiller les LOG (FLASH_DONE / FLASH_FAILED)")
