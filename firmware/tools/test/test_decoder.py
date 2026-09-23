from coffeetool.decoder import decode_frame
from coffeetool.framing import RawFrame
from coffeetool.messages import LogPayload, PongPayload, StatusHeatingPayload
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
        data=LogPayload(code=6, severity=3, arg16=0, arg32=60000).pack(),
    )
    line = decode_frame(0.0, frame)
    assert "RUNTIME_LOCKOUT_TRIGGERED" in line
    assert "error" in line
    assert "arg32=60000" in line


def test_decode_unknown_type_does_not_crash():
    frame = RawFrame(can_id=0x7FF, data=b"\xff" * 8)
    line = decode_frame(0.0, frame)
    assert "inconnu" in line


def test_decode_new_heating_and_confirmation_frames():
    heating = decode_frame(0.0, RawFrame(0x46A, StatusHeatingPayload(False, 0).pack()[:4]))
    assert "STATUS_HEATING" in heating
    assert "chauffage=off" in heating
    assert "capable=oui" in heating

    confirmation = decode_frame(0.0, RawFrame(0x0B1, b"\x3f"))
    assert "CONFIRM_SENSORS_OTA" in confirmation
    assert "version_protocole_chauffage=63" in confirmation

    request = decode_frame(0.0, RawFrame(0x211, b"\x23\xfa\x00"))
    assert "cible=STATUS_HEATING" in request
