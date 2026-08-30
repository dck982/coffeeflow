import time
from machine import Pin, UART

# Atom Echo S3R (C126-ECHO):
#   PORT.A Grove: yellow=G2, white=G1 (these are the documented GPIOs)
#   Rear header: G5 G6 G7 G8 G38 G39 — only if you are on those holes
# USB REPL is UART0. UART1 is unused until a script opens it.

try:
    UART(1).deinit()
except OSError:
    pass

BUTTON = 41
GROVE = (1, 2)
HEADER = (5, 6, 7, 8, 38, 39)


def report(n, mode, extra=""):
    print("  GPIO {:2d}  {} = {}{}".format(n, mode, Pin(n, Pin.IN).value(), extra))


print("1) pull-up (expect 1 unless the pad is shorted to GND)")
for n in GROVE + HEADER:
    Pin(n, Pin.IN, Pin.PULL_UP)
    time.sleep_ms(5)
    v = Pin(n, Pin.IN, Pin.PULL_UP).value()
    print("  GPIO {:2d}  PULL_UP = {}{}".format(n, v, "" if v else "  STUCK LOW"))

print("2) button G41 (press it, expect 0)")
Pin(BUTTON, Pin.IN, Pin.PULL_UP)
print("  GPIO 41  now = {}".format(Pin(BUTTON, Pin.IN).value()))

print("3) toggling Grove G2 (yellow) 1 Hz — meter on Grove yellow vs Grove black (GND)")
p = Pin(2, Pin.OUT)
while True:
    p.value(1)
    print("G2 HIGH")
    time.sleep(1)
    p.value(0)
    print("G2 LOW")
    time.sleep(1)
