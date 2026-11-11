import time
from machine import Pin

# PUMP_TACHO on GPIO3: U9 pulls it low against R39 (4.7 kΩ to 3V3)
# while J9 blue is low. Bench test: J9 pin 4 on +5V, pin 1 on GND.
# Counts falling edges; a hand-made contact bounces, so one touch can
# give several edges.
DURATION_S = 30

count = 0


def on_edge(pin):
    global count
    count += 1


tacho = Pin(3, Pin.IN)
tacho.irq(trigger=Pin.IRQ_FALLING, handler=on_edge)

print("counting for %d s, idle level %d" % (DURATION_S, tacho.value()))
last = 0
for s in range(DURATION_S):
    time.sleep(1)
    c = count
    if c != last:
        print("%2d s: %d edges (+%d)" % (s + 1, c, c - last))
        last = c
tacho.irq(handler=None)
print("total: %d edges" % count)
