from machine import I2C, Pin

TMP102 = 0x48
ADS1115 = 0x49

i2c = I2C(0, sda=Pin(5), scl=Pin(6), freq=100_000)

found = i2c.scan()
print("scan:", [hex(a) for a in found])

if TMP102 in found:
    raw = i2c.readfrom_mem(TMP102, 0x00, 2)
    t = ((raw[0] << 8 | raw[1]) >> 4)
    if t & 0x800:
        t -= 0x1000
    print("TMP102 0x48: %.2f °C" % (t * 0.0625))
else:
    print("TMP102 0x48: absent")

if ADS1115 in found:
    cfg = i2c.readfrom_mem(ADS1115, 0x01, 2)
    # 0x8583 is the power-on default of the config register
    print("ADS1115 0x49: config 0x%04x" % (cfg[0] << 8 | cfg[1]))
else:
    print("ADS1115 0x49: absent")
