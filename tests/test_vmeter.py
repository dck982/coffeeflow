import time
from machine import Pin, SoftI2C

# Unit VMeter (U087) on Atom Echo S3R PORT.A.
# Grove: yellow=SDA=G2, white=SCL=G1.
# Measure on the HT3.96 screws (V+ / V-), not the Grove wires.
# Range ±36 V DC. Do not write EEPROM 0x53 (factory cal).

i2c = SoftI2C(
    sda=Pin(2, Pin.OPEN_DRAIN, Pin.PULL_UP),
    scl=Pin(1, Pin.OPEN_DRAIN, Pin.PULL_UP),
    freq=100_000,
)

ADS = 0x49
EEPROM = 0x53
DIVIDER = 0.015918958  # ADC mV per input mV
LSB_MV = 0.015625  # PGA ±0.512 V
PGA = 4  # ADS1115_PGA_512, 16 V input
EEPROM_REG = 0xD0 + PGA * 8  # 0xF0

found = i2c.scan()
print("I2C:", [hex(a) for a in found])
if ADS not in found:
    raise RuntimeError("VMeter 0x49 introuvable")


def w16(addr, reg, val):
    i2c.writeto_mem(addr, reg, bytes([(val >> 8) & 0xFF, val & 0xFF]))


def r16(addr, reg):
    b = i2c.readfrom_mem(addr, reg, 2)
    return (b[0] << 8) | b[1]


def i16(u):
    return u - 0x10000 if u >= 0x8000 else u


def factory_cal():
    raw = i2c.readfrom_mem(EEPROM, EEPROM_REG, 8)
    chk = 0
    for b in raw[:5]:
        chk ^= b
    if chk != raw[5]:
        print("EEPROM cal invalid, factor=1")
        return 1.0
    hope = i16((raw[1] << 8) | raw[2])
    actual = i16((raw[3] << 8) | raw[4])
    if actual == 0:
        return 1.0
    factor = abs(hope / actual)
    print("cal factor={:.4f}".format(factor))
    return factor


cal = factory_cal()
# OS=1 MUX=A0-A1 PGA=512 MODE=single DR=128 COMP=off
CONFIG = 0x8983


def read_mv():
    w16(ADS, 0x01, CONFIG)
    time.sleep_ms(20)
    adc = -i16(r16(ADS, 0x00))
    return adc * (LSB_MV / DIVIDER) * cal


while True:
    mv = read_mv()
    print("{:.1f} mV  ({:.3f} V)".format(mv, mv / 1000.0))
    time.sleep(0.3)
