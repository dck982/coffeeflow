import time
from machine import CAN, Pin

# Atom S3 Port.A (HY2.0) + Unit CAN (CA-IS3050G).
# Grove: noir=GND, rouge=5V, jaune=G2, blanc=G1.
# Unit CAN: jaune=TX, blanc=RX → TWAI tx=G2, rx=G1.
# Set SWAP = True if TX/RX are crossed.
# Bus coffeeflow: 500 kbit/s. Change BITRATE if needed (e.g. 125_000).

SWAP = False
BITRATE = 500_000

if SWAP:
    TX_PIN = 1
    RX_PIN = 2
else:
    TX_PIN = 2
    RX_PIN = 1

# LISTEN_ONLY: receive without ACKing (bus sniffer).
# Fall back to NORMAL if the firmware has no LISTEN_ONLY.
try:
    mode = CAN.LISTEN_ONLY
except AttributeError:
    mode = CAN.NORMAL

can = CAN(0, tx=TX_PIN, rx=RX_PIN, mode=mode, bitrate=BITRATE)

print(
    "CAN sniffer  {} bit/s  TX=G{} RX=G{}  mode={}".format(
        BITRATE, TX_PIN, RX_PIN, mode
    )
)
print("waiting for frames…")


def fmt_data(data):
    return " ".join("{:02X}".format(b) for b in data)


while True:
    if can.any():
        msg = can.recv()
        # MicroPython: (id, is_extended, is_rtr, data)
        can_id, ext, rtr, data = msg[0], msg[1], msg[2], msg[3]
        kind = "EXT" if ext else "STD"
        if rtr:
            print("{} RTR id=0x{:03X} dlc={}".format(kind, can_id, len(data)))
        else:
            print(
                "{} id=0x{:03X} dlc={} data=[{}]".format(
                    kind, can_id, len(data), fmt_data(data)
                )
            )
    else:
        time.sleep_ms(1)
