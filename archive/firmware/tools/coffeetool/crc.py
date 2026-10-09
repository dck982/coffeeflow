"""CRC16-CCITT et CRC32 — portage à la main de firmware/common/src/crc.cpp.

Aucun code partagé avec le C++ (contrairement à log_codes, qui est généré) :
chaque côté a ses propres vecteurs de test, voir test/test_crc.py et
firmware/common/test/test_common.cpp.
"""


def crc16_ccitt(data: bytes) -> int:
    """Poly 0x1021, init 0xFFFF — utilisé pour l'acquittement de bloc du flash."""
    crc = 0xFFFF
    for byte in data:
        crc ^= byte << 8
        for _ in range(8):
            if crc & 0x8000:
                crc = ((crc << 1) ^ 0x1021) & 0xFFFF
            else:
                crc = (crc << 1) & 0xFFFF
    return crc


def crc32_ieee(data: bytes) -> int:
    """Poly 0xEDB88320, init/xorout 0xFFFFFFFF (variante zlib/IEEE 802.3)."""
    crc = 0xFFFFFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            mask = -(crc & 1) & 0xFFFFFFFF
            crc = (crc >> 1) ^ (0xEDB88320 & mask)
    return crc ^ 0xFFFFFFFF
