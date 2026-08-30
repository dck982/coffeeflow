import time
from machine import Pin, UART

# Jumper G6 to G7, then run this.
# One pin OUT, one pin IN — never both OUT.

try:
    UART(1).deinit()
except OSError:
    pass

DRIVE = 6
READ = 7

rx = Pin(READ, Pin.IN)
tx = Pin(DRIVE, Pin.OUT)

print("jumper GPIO {} -> GPIO {}".format(DRIVE, READ))

for v in (0, 1, 0, 1):
    tx.value(v)
    time.sleep_ms(20)
    got = rx.value()
    print("out {}  in {}  {}".format(v, got, "OK" if got == v else "FAIL"))
