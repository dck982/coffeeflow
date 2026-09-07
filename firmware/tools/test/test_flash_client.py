from coffeetool.crc import crc16_ccitt
from coffeetool.flash_client import BLOCK_SIZE, FlashError, flash
from coffeetool.framing import RawFrame
from coffeetool.messages import FlashCtrlPayload, FlashSubCmd
from coffeetool.protocol import CanId, Dest, MessageType, Node, encode_can_id


class _FakeReceiver:
    """Simule le module capteurs côté réception sur le chemin nominal
    (docs/firmware.md, "Flash — le seul cas de réassemblage") : accuse
    chaque bloc complet avec son CRC16 réel. Ne modélise pas la
    renégociation d'un bloc après un NAK — ce n'est pas fixé côté firmware
    avant la phase 4 (voir messages.hpp, FlashCtrlPayload)."""

    def __init__(self, ctrl_id: int):
        self.ctrl_id = ctrl_id
        self.received = bytearray()
        self.image_size = None
        self.pending = []

    def send(self, frame: RawFrame) -> None:
        message_type = (frame.can_id >> 5) & 0x3F
        if message_type == int(MessageType.FLASH_CTRL):
            ctrl = FlashCtrlPayload.unpack(frame.data)
            if ctrl.subcmd == FlashSubCmd.BEGIN:
                self.image_size = ctrl.image_size
                self.received.clear()
                self._ack(FlashCtrlPayload(subcmd=FlashSubCmd.BLOCK_ACK, block_number=0))
            elif ctrl.subcmd == FlashSubCmd.END:
                self._ack(FlashCtrlPayload(subcmd=FlashSubCmd.END, image_crc32=ctrl.image_crc32))
        elif message_type == int(MessageType.FLASH_DATA):
            self.received.extend(frame.data)
            if len(self.received) % BLOCK_SIZE == 0 or len(self.received) == self.image_size:
                block_number = (len(self.received) - 1) // BLOCK_SIZE + 1
                start = (block_number - 1) * BLOCK_SIZE
                block = bytes(self.received[start:])
                self._ack(
                    FlashCtrlPayload(
                        subcmd=FlashSubCmd.BLOCK_ACK,
                        block_number=block_number,
                        block_crc16=crc16_ccitt(block),
                    )
                )

    def _ack(self, payload: FlashCtrlPayload) -> None:
        self.pending.append((1.0, RawFrame(self.ctrl_id, payload.pack())))

    def recv(self, timeout=None):
        return self.pending.pop(0) if self.pending else None


class _NeverAckingReceiver:
    def send(self, frame: RawFrame) -> None:
        pass

    def recv(self, timeout=None):
        return None


def test_flash_small_image_succeeds():
    ctrl_id = encode_can_id(CanId(MessageType.FLASH_CTRL, Dest.SCREEN, Node.SENSORS))
    receiver = _FakeReceiver(ctrl_id)
    image = bytes(range(256)) * 4  # 1024 octets, un seul bloc partiel

    flash(receiver, image, src=Node.SCREEN, dest=Dest.SENSORS)

    assert bytes(receiver.received) == image


def test_flash_gives_up_without_begin_ack():
    try:
        flash(_NeverAckingReceiver(), b"\x00" * 16, src=Node.SCREEN, dest=Dest.SENSORS)
        assert False, "devait lever FlashError"
    except FlashError as exc:
        assert "BEGIN" in str(exc)
