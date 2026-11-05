import time
from machine import Pin, UART

# XKC-Y26A-NPN non-contact liquid level sensor on an AtomS3.
#   brown  -> 5V
#   blue   -> GND
#   yellow -> G8, pulled up to 3.3V through 4.7k (open-collector output)
#   black  -> floating: normally open, the transistor conducts on liquid
#
# Floating black wire: liquid pulls G8 low (0), no liquid leaves it high (1).
# The sensor takes about 500 ms to settle after a change.

try:
    UART(1).deinit()
except OSError:
    pass

SENSE = 8

pin = Pin(SENSE, Pin.IN)

print("XKC-Y26A-NPN on GPIO {}  (Ctrl+C to stop)".format(SENSE))

last = None
while True:
    level = pin.value()
    state = "no liquid" if level else "LIQUID"
    mark = "" if level == last else "  <- change"
    print("G{}={}  {}{}".format(SENSE, level, state, mark))
    last = level
    time.sleep(0.25)
