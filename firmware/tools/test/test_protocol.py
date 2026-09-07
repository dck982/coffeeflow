from coffeetool.protocol import CanId, Dest, MessageType, Node, decode_can_id, encode_can_id, is_known_message_type


def test_can_id_roundtrip():
    can_id = CanId(MessageType.SET, Dest.SENSORS, Node.SCREEN)
    raw = encode_can_id(can_id)
    assert raw <= 0x7FF
    back = decode_can_id(raw)
    assert back.type == can_id.type
    assert back.dest == can_id.dest
    assert back.src == can_id.src


def test_can_id_priority_ordering():
    # STOP (0x00) doit gagner l'arbitrage face à FLASH_DATA (0x39).
    stop = encode_can_id(CanId(MessageType.STOP, Dest.SENSORS, Node.SCREEN))
    flash = encode_can_id(CanId(MessageType.FLASH_DATA, Dest.SENSORS, Node.SCREEN))
    assert stop < flash


def test_known_message_type():
    assert is_known_message_type(int(MessageType.PING))
    assert not is_known_message_type(0x3F)
