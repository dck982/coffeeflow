import time
from machine import Pin

# HEATER_CMD on GPIO4, active HIGH: J8 pin 2 goes to +5V.
ON_S = 15

boiler = Pin(4, Pin.OUT, value=0)
print("BOILER on for %d s" % ON_S)
boiler.value(1)
for s in range(ON_S, 0, -1):
    print(s)
    time.sleep(1)
boiler.value(0)
print("BOILER off")
