import time
from machine import Pin

# Digmesa FHKSC : collecteur ouvert NPN sur G38.
# Alim capteur en 5 V + GND. Pull-up interne OK pour un banc court (~5 cm).
#pulse = Pin(38, Pin.IN, Pin.PULL_UP)
pulse = Pin(38, Pin.IN)

count = 0


def on_pulse(_pin):
    global count
    count += 1


pulse.irq(trigger=Pin.IRQ_FALLING, handler=on_pulse)

print("flowmeter G38, pull-up interne, front descendant")

WINDOW = 10
deltas = [0] * WINDOW
idx = 0
prev = 0

while True:
    n = count
    delta = n - prev
    prev = n
    deltas[idx] = delta
    idx = (idx + 1) % WINDOW
    window = sum(deltas)
    print("total={}  window10={}".format(n, window))
    time.sleep(0.1)
