import time
from machine import Pin

# SSR (valve) on GPIO9, active HIGH: Q1 lights D1 and the MOC3043 LED.
# Run with nothing on J2: the triac side stays unpowered.
ssr = Pin(9, Pin.OUT, value=0)
for i in range(5):
    print("D1 on", i + 1)
    ssr.value(1)
    time.sleep(1)
    ssr.value(0)
    time.sleep(1)
print("D1 off")
