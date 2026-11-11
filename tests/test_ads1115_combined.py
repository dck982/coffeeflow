import time
from machine import I2C, Pin

ADS1115 = 0x49
MUX = {0: 0x4, 1: 0x5}  # AIN0 / AIN1 against GND
LSB = 4.096 / 32768  # PGA ±4.096 V


def read(ch):
    # single-shot, PGA ±4.096 V, 128 SPS, comparator off
    cfg = 0x8000 | MUX[ch] << 12 | 0x1 << 9 | 0x1 << 8 | 0x4 << 5 | 0x3
    i2c.writeto_mem(ADS1115, 0x01, bytes([cfg >> 8, cfg & 0xFF]))
    time.sleep_ms(10)
    raw = i2c.readfrom_mem(ADS1115, 0x00, 2)
    v = raw[0] << 8 | raw[1]
    if v & 0x8000:
        v -= 0x10000
    return v * LSB


i2c = I2C(0, sda=Pin(5), scl=Pin(6), freq=100_000)

for ch, name in ((0, "AIN0 +3V3"), (1, "AIN1 PRESSURE, nothing on J10")):
    vals = [read(ch) for _ in range(8)]
    print("%s: %.4f V (min %.4f, max %.4f)" % (name, sum(vals) / 8, min(vals), max(vals)))
