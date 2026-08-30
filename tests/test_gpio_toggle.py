import time
from machine import Pin, UART

# Atom Echo S3R rear 5-pin (USB-C at bottom): 3V3, G5, G6, G7, G8.
# UART1 is only on G7/G8 after test_rbuart.py; USB REPL is UART0, not these pins.
try:
    UART(1).deinit()
except OSError:
    pass

# Change this to 5, 6, 7 or 8 to find which hole is which.
PIN = 6

p = Pin(PIN, Pin.OUT)
print("toggling GPIO {} every 1 s (1 = 3.3 V, 0 = 0 V)".format(PIN))

while True:
    p.value(1)
    print("HIGH")
    time.sleep(1)
    p.value(0)
    print("LOW")
    time.sleep(1)
