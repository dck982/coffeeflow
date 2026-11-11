import time
from machine import Pin

# FLOW_PULSE on GPIO44, 1 kΩ pull-up to 3V3 and 10 nF on the board.
# Counts falling edges; a hand-made contact bounces, so one touch can
# give several edges.
DURATION_S = 30

count = 0


def on_edge(pin):
    global count
    count += 1


flow = Pin(44, Pin.IN)
flow.irq(trigger=Pin.IRQ_FALLING, handler=on_edge)

print("counting for %d s, idle level %d" % (DURATION_S, flow.value()))
last = 0
for s in range(DURATION_S):
    time.sleep(1)
    c = count
    if c != last:
        print("%2d s: %d edges (+%d)" % (s + 1, c, c - last))
        last = c
flow.irq(handler=None)
print("total: %d edges" % count)
