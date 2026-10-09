from coffeetool.crc import crc16_ccitt, crc32_ieee


def test_crc16_known_vector():
    # Même vecteur que firmware/common/test/test_common.cpp.
    assert crc16_ccitt(b"123456789") == 0x29B1


def test_crc32_known_vector():
    assert crc32_ieee(b"123456789") == 0xCBF43926
