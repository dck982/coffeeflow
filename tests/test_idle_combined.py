import time
from machine import Pin

# Inputs with nothing on J6, J9 or J1: all pulled high.
for name, gpio, expect in (
    ("FLOW_PULSE (R pull-up 3V3)", 44, 1),
    ("PUMP_TACHO (R39 pull-up, U9 off)", 3, 1),
    ("CAN RX (bus recessive)", 8, 1),
):
    v = Pin(gpio, Pin.IN).value()
    print("%-34s GPIO%-2d = %d %s" % (name, gpio, v, "ok" if v == expect else "UNEXPECTED"))

# CAN transceiver loopback: TXD low drives the bus dominant, RXD follows.
# The TJA1051 releases the bus after its TXD dominant time-out (>= 0.3 ms),
# so RXD is read right away.
tx = Pin(7, Pin.OUT, value=1)
rx = Pin(8, Pin.IN)
time.sleep_ms(1)
rec = rx.value()
tx.value(0)
dom = rx.value()
tx.value(1)
time.sleep_ms(1)
back = rx.value()
Pin(7, Pin.IN)
ok = (rec, dom, back) == (1, 0, 1)
print("CAN loopback TX->RX: recessive %d, dominant %d, released %d %s"
      % (rec, dom, back, "ok" if ok else "UNEXPECTED"))
