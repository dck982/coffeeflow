from coffeetool.messages import (
    FlashCtrlPayload,
    FlashSubCmd,
    LogPayload,
    PongPayload,
    ReqStatusPayload,
    SetPayload,
    StatusActuatorsPayload,
    StatusFlowPayload,
    StatusPressurePayload,
)
from coffeetool.protocol import MessageType, Node


def test_set_payload_roundtrip():
    original = SetPayload(set_ssr=True, set_dimmer=True, ssr=True, dimmer=42, ttl_ms=500)
    out = SetPayload.unpack(original.pack())
    assert out == original


def test_pong_payload_roundtrip():
    original = PongPayload(Node.SENSORS, 1, 2, 3, 123456789)
    out = PongPayload.unpack(original.pack())
    assert out == original


def test_reqstatus_payload_roundtrip():
    original = ReqStatusPayload(MessageType.STATUS_FLOW, 200)
    out = ReqStatusPayload.unpack(original.pack())
    assert out == original


def test_status_pressure_payload_roundtrip():
    original = StatusPressurePayload(0xABCDEF & 0xFFFFFF, 0x1234, 60000, 0b01)
    out = StatusPressurePayload.unpack(original.pack())
    assert out == original


def test_status_flow_payload_roundtrip():
    original = StatusFlowPayload(4_000_000_000, 1234, 1)
    out = StatusFlowPayload.unpack(original.pack())
    assert out == original


def test_status_actuators_payload_roundtrip():
    original = StatusActuatorsPayload(True, 77, 480, 59000, 0b101)
    out = StatusActuatorsPayload.unpack(original.pack())
    assert out == original


def test_log_payload_roundtrip():
    original = LogPayload(code=5, severity=3, arg16=0, arg32=60000)
    out = LogPayload.unpack(original.pack())
    assert out == original


def test_flash_ctrl_payload_roundtrip():
    begin = FlashCtrlPayload(subcmd=FlashSubCmd.BEGIN, image_size=1048576)
    assert FlashCtrlPayload.unpack(begin.pack()) == begin

    block_ack = FlashCtrlPayload(subcmd=FlashSubCmd.BLOCK_ACK, block_number=42, block_crc16=0xBEEF)
    assert FlashCtrlPayload.unpack(block_ack.pack()) == block_ack

    end = FlashCtrlPayload(subcmd=FlashSubCmd.END, image_crc32=0xDEADBEEF)
    assert FlashCtrlPayload.unpack(end.pack()) == end


def test_all_payloads_pack_to_eight_bytes():
    for payload in (
        SetPayload(),
        PongPayload(),
        ReqStatusPayload(),
        StatusPressurePayload(),
        StatusFlowPayload(),
        StatusActuatorsPayload(),
        LogPayload(),
        FlashCtrlPayload(),
    ):
        assert len(payload.pack()) == 8
