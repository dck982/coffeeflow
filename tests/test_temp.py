import time
from machine import Pin, SoftI2C

ADDR = 0x66
REG_TEMP = 0x00  # thermocouple, 4 octets, 0.01 °C

i2c = SoftI2C(scl=Pin(1), sda=Pin(2), freq=100_000)

found = i2c.scan()
print("I2C:", [hex(a) for a in found])
if ADDR not in found:
    raise RuntimeError("KISOMeter 0x66 introuvable")


def temp_c():
    raw = i2c.readfrom_mem(ADDR, REG_TEMP, 4)
    val = int.from_bytes(raw, "little")
    if val >= 0x80000000:
        val -= 0x100000000
    return val / 100.0


while True:
    print("{:.2f} C".format(temp_c()))
    time.sleep(1)
