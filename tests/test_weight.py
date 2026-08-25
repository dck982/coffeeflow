import struct
import time
from machine import Pin, SoftI2C

ADDR = 0x26
REG_RAW_ADC = 0x00       # int32 LE
REG_WEIGHT = 0x10         # float LE, grammes
REG_WEIGHT_X100 = 0x60    # int32 LE, poids = valeur / 100
REG_WEIGHT_STR = 0x70     # string NUL, max 15 chars
REG_FILTER = 0x80         # 3 octets: lp, avg, ema

i2c = SoftI2C(scl=Pin(1), sda=Pin(2), freq=100_000)

found = i2c.scan()
print("I2C:", [hex(a) for a in found])
if ADDR not in found:
    raise RuntimeError("Unit Weight 0x26 introuvable")


def i32_le(raw):
    val = int.from_bytes(raw, "little")
    if val >= 0x80000000:
        val -= 0x100000000
    return val


def raw_adc():
    return i32_le(i2c.readfrom_mem(ADDR, REG_RAW_ADC, 4))


def weight_float_g():
    return struct.unpack("<f", i2c.readfrom_mem(ADDR, REG_WEIGHT, 4))[0]


def weight_x100():
    return i32_le(i2c.readfrom_mem(ADDR, REG_WEIGHT_X100, 4))


def weight_str():
    raw = i2c.readfrom_mem(ADDR, REG_WEIGHT_STR, 16)
    end = raw.find(b"\x00")
    if end < 0:
        end = len(raw)
    return raw[:end].decode()


filt = i2c.readfrom_mem(ADDR, REG_FILTER, 3)
print("filters: lp={} avg={} ema={}".format(filt[0], filt[1], filt[2]))

while True:
    x100 = weight_x100()
    print(
        "raw={}  float={:.2f} g  int={:.2f} g (x100={})  str={!r}".format(
            raw_adc(),
            weight_float_g(),
            x100 / 100.0,
            x100,
            weight_str(),
        )
    )
    time.sleep(0.1)
