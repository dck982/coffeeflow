from coffeetool.decoder import decode_frame
from coffeetool.framing import RawFrame
from coffeetool.messages import LogPayload, PongPayload
from coffeetool.protocol import CanId, Dest, MessageType, Node, encode_can_id


def test_decode_pong():
    frame = RawFrame(
        can_id=encode_can_id(CanId(MessageType.PONG, Dest.SCREEN, Node.SENSORS)),
        data=PongPayload(Node.SENSORS, 0, 1, 0, 42).pack(),
    )
    line = decode_frame(0.0, frame)
    assert "PONG" in line
    assert "v0.1.0" in line
    assert "uptime=42s" in line


def test_decode_log_uses_generated_code_names():
    frame = RawFrame(
        can_id=encode_can_id(CanId(MessageType.LOG, Dest.SCREEN, Node.SENSORS)),
        data=LogPayload(code=5, severity=3, arg16=0, arg32=60000).pack(),
    )
    line = decode_frame(0.0, frame)
    assert "RUNTIME_LOCKOUT_TRIGGERED" in line
    assert "error" in line
    assert "arg32=60000" in line


def test_decode_unknown_type_does_not_crash():
    frame = RawFrame(can_id=0x7FF, data=b"\xff" * 8)
    line = decode_frame(0.0, frame)
    assert "inconnu" in line
