import time
from machine import Pin, UART

# Listen only. Open this first, then power-cycle DimmerLink VCC (3.3 V, not 5 V).
# You should see ASCII like "=== DimmerLink" / "Mode: UART".
#
# Module TX -> G7 (ESP32 RX). Module RX -> G8 (ESP32 TX).
# Set SWAP = True if you crossed the Duponts the other way.

SWAP = False

if SWAP:
    uart = UART(1, baudrate=115200, bits=8, parity=None, stop=1, tx=Pin(7), rx=Pin(8))
    print("RX=G8  TX=G7")
else:
    uart = UART(1, baudrate=115200, bits=8, parity=None, stop=1, tx=Pin(8), rx=Pin(7))
    print("RX=G7  TX=G8")

print("listening — power-cycle DimmerLink VCC now")

while True:
    n = uart.any()
    if n:
        data = uart.read(n)
        print("hex", data)
        try:
            print("txt", data.decode())
        except UnicodeError:
            pass
    time.sleep_ms(50)
