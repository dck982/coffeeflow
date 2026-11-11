import time
from machine import Pin

# Standalone valve test for the combined board, runs without USB.
# boot.py blinks D4 for 5 s, then this waits 5 s more with the valve off
# (the lamp on J3 must stay dark at power-up), then cycles the valve
# 3 s on / 3 s off forever. D4 follows the valve command.
ON_S = 3
OFF_S = 3

ssr = Pin(9, Pin.OUT, value=0)
led = Pin(2, Pin.OUT, value=0)
time.sleep(5)
while True:
    ssr.value(1)
    led.value(1)
    time.sleep(ON_S)
    ssr.value(0)
    led.value(0)
    time.sleep(OFF_S)
