from machine import Pin, SoftI2C
import time

i2c = SoftI2C(sda=Pin(7, Pin.OPEN_DRAIN, Pin.PULL_UP),
              scl=Pin(8, Pin.OPEN_DRAIN, Pin.PULL_UP),
              freq=50000)

devices = i2c.scan()

if not devices:
    print("Aucun périphérique détecté")
else:
    for addr in devices:
        print("Trouvé : {} (0x{:02X})".format(addr, addr))
