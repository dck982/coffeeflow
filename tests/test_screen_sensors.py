from machine import Pin, SoftI2C
import time

# pcb/screen_sensors through pcb/i2cterm, from an Atom S3 (SDA G2, SCL G1).
# Reads the SHT40 (0x44) and the four ADS1115 (0x48) inputs.
# No internal pull-ups: i2cterm provides the 4.7 kΩ.
i2c = SoftI2C(sda=Pin(2, Pin.OPEN_DRAIN), scl=Pin(1, Pin.OPEN_DRAIN), freq=50000)

print("scan:", [hex(a) for a in i2c.scan()])


def crc8(data):
    crc = 0xFF
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = ((crc << 1) ^ 0x31) & 0xFF if crc & 0x80 else (crc << 1) & 0xFF
    return crc


# SHT40 @0x44
i2c.writeto(0x44, b"\x89")
time.sleep_ms(2)
s = i2c.readfrom(0x44, 6)
print("SHT40 serial ok CRC:", crc8(s[0:2]) == s[2] and crc8(s[3:5]) == s[5])
i2c.writeto(0x44, b"\xfd")
time.sleep_ms(10)
d = i2c.readfrom(0x44, 6)
print("SHT40 CRC:", crc8(d[0:2]) == d[2] and crc8(d[3:5]) == d[5])
t = -45 + 175 * (d[0] << 8 | d[1]) / 65535
rh = -6 + 125 * (d[3] << 8 | d[4]) / 65535
print("SHT40 T=%.2f C  RH=%.1f %%" % (t, rh))

# ADS1115 @0x48, PGA +/-4.096 V, 128 SPS, single-shot
for ch, name in enumerate(("A0 3V3", "A1 diviseur chaudiere", "A2 pression", "A3 libre")):
    cfg = 0x8000 | ((4 + ch) << 12) | 0x0200 | 0x0100 | 0x0080 | 0x0003
    i2c.writeto_mem(0x48, 0x01, cfg.to_bytes(2, "big"))
    time.sleep_ms(15)
    raw = int.from_bytes(i2c.readfrom_mem(0x48, 0x00, 2), "big")
    if raw & 0x8000:
        raw -= 0x10000
    print("ADS1115 %-22s raw=%6d  %.4f V" % (name, raw, raw * 125e-6))
